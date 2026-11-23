# Command names (name map renames, 2026-11-07)

Survey of every command-line / script / MCP name Operandi accepts, against
`TOOLBAR-NAME-MAP.md`, `modo-tools.json` and `modo-commands.json`. "Map" below
means those three files. Renames live in one registry:
`crates/operandi-core/src/command_names.rs`. Old names stay accepted and log
`Deprecated command name: <old> is now <new>` once per name per run.

Entry points surveyed: command line and every caller of `Operandi::execute`
(menus, toolbox, modeling menu, pie entries, input bindings, workspace
command field), `tool.set/attr/reset/apply/drop`, the vertex, edge, backdrop,
selection set and item list sub-parsers, the MCP bridge (`tool_run`,
`command_run`), the input map file (stores action labels, not command
strings; unchanged), pie menus (store Operandi phrases like `View Top`, not
map commands; unchanged), toolbox config (labels only).

## Tools (`tool.set <id>`)

| Operandi (old) | Map | Status | Arg renames |
|---|---|---|---|
| prim.cube / sphere / cylinder / ellipsoid / cone | same | already matches | – |
| prim.capsule / arc / tube / pen / text | same | already matches | – |
| **prim.torus** | prim.toroid | **renamed**, old aliased | – |
| poly.bevel / extrude / smshift / spikey / reduct / haul / sweep / inset / shift | same | already matches | poly.reduct: **legacy → method** |
| poly.twistBevel | – (not in the map) | Operandi's own, the spec's name (twist-bevel-2026-11-15) | – |
| chamfer.edit | same | already matches | **sourceSegments → sourceSegs** (one-shot `chamfer.edit`) |
| Thicken | Thicken (preset) | already matches | – |
| edge.bridge / extend / extrude / slide / bevel / chamfer / relax / addLoop / addPoint | same | already matches | edge.bevel, edge.chamfer: **useMaterial → useMat, material → matName** |
| vert.bevel / vert.extrude | same | already matches | – |
| CurveExtrude / BezierExtrude / BSplineExtrude, CurveClone / BezierClone / BSplineClone / PenClone (presets) | – (not in the map; CX-Q6) | Operandi's own ids for the path tool buttons (2026-11-17) | – |
| **vert.create** | prim.makeVertex | **renamed**, old aliased | – |
| **vert.mergeTool** | vert.merge (tool) | **renamed**, old aliased (`tool.set vert.merge` is the tool; bare `vert.merge` stays the command) | – |
| **vert.weld** (tool.set) | DragWeld (preset) | **renamed**, old aliased | – |
| vert.slide | vert.Slide via `*.Slide` | kept (ids are case-insensitive); **`*.Slide` added**: Vertex mode → vert.slide, Edge mode → edge.slide, other modes refused (provisional, no Polygon/Item Slide) | – |
| vert.tear | `@ppi_vert_tear.py` (a script command, not a tool) | tool id kept; **`@ppi_vert_tear.py` added** as a command that starts Tear | – |
| ElementMove (tweak) | ElementMove (preset) | renamed earlier; `tweak` alias now goes through the registry | – |
| **deform.push** | xfrm.push | **renamed**, old aliased | – |
| **deform.smooth** | xfrm.smooth | **renamed**, old aliased | – |
| xfrm.jitter, xfrm.quantize, xfrm.radialAlign, xfrm.linearAlign (new, deform finish) | same (button invocations) | new; `tool.set <id> on\|off`; buttons "Jitter", "Quantize", "Radial Align" ("Radial"), "Linear Align" ("Linear") | tool attrs are the panel keys (`jitter_range0`, `radial_mode` ...); reference attr names not mapped yet |
| xfrm.vortex (new preset) | same | new preset row; "Vortex" | – |
| select.shortestPath, select.edgeSharp, select.edgeCoplanar, select.vertexColinear, select.edgeDiagonal, select.conditionalLoop, select.lazyState (new) | same | new; bare "..." commands open their dialogs; "Shortest Path Mode" is Operandi's toggle | edgeSharp `min max`, edgeCoplanar `flatness`, vertexColinear `distance`, edgeDiagonal `flatness max_angle keep_*` (Operandi names, SL spec gives none) |
| tool.set select.polygonCoplanar / seam.marker | Coplanar Polygons Tool / Conditional Loop Tool | dialogs for now (SL-Q3/Q4) | – |
| sculpt.relaxMesh (new) | same | bare opens the dialog, like the button "Relax Mesh..." | `iter`, `constrain` |
| **slice.loop** | poly.loopSlice | **renamed**, old aliased | – |
| **slice.edge** | edge.knife (Edge Slice) | **renamed**, old aliased | – |
| **slice.polygon** | poly.knife (Slice) | **renamed**, old aliased | – |
| Transform, TransformMove, TransformRotate, TransformScale | no toolbar row (reference preset names) | kept | – |
| edge.crease (Edge Weight) | no match | kept | – |
| sculpt.smooth / push / inflate / move (new, sculpt-foundation) | same (spec section 14) | new; `tool.set sculpt.push on\|off\|?`, bare `sculpt.push` toggles | none yet (argument names are [check] in the spec) |
| sculpt.fold / clay / smudge / carve / scar / flatten / tangentPinch / centerPinch / emboss / shrink (Attenuate) / spin (new, sculpt-toolbox) | same (NAMES.txt addendum) | new; `tool.set sculpt.clay on\|off\|?`, bare names toggle; input-map actions "Sculpt Clay" etc. | form field names (oval, level, flat, moveMode, …) listed in NAMES.txt, not yet arguments |
| sculpt.vmapPaint / vmapSmooth / vmapScale / vmapErase / vmapSetVal, sculpt.liveWeightEdit, vertMap.new, select.vertexMap, falloff.vertexMap, falloff.invert (new, vertex-map-tools) | registry reply 2026-11-11; in MODO-NAMES-SUBSET.txt | `tool.set sculpt.vmapPaint on\|off\|?`; `tool.set falloff.vertexMap on\|off` adds/removes; "Vertex Map Paint" etc. run tool.set | `vertMap.new name type init color value`, `select.vertexMap name type mode`, `falloff.vertexMap mode index mix` |
| falloff.selection (new), Flex (preset) | registry reply 2026-11-13; falloff.selection in MODO-NAMES-SUBSET.txt, Flex in the button invocations | `tool.set falloff.selection on\|off`; `tool.set Flex on` and "Flex" start the preset | `falloff.selection steps shape p0 p1 mix` (shape linear/easeIn/easeOut/smooth/custom = 0..4, default smooth) |
| xfrm.bend (new), Bend (preset) | registry reply 2026-11-13; xfrm.bend in MODO-NAMES-SUBSET.txt, Bend in the button invocations | `tool.set xfrm.bend on`, `tool.set Bend on` and "Bend" start it | `tool.attr xfrm.bend angle\|spineX\|spineY\|spineZ\|bothSides`; lockUV and morph not built |
| Flare (preset) | tool-presets spec v0.2, reference preset name | `tool.set Flare on`, "Flare" and "flare" start it (row in presets/factory-tool-presets.toml) | Push's attributes |
| FlipIt, FlipItOrigin (Operandi's own) | tool-presets spec v0.2 [OP] | "Flip It [x\|y\|z]", "Flip It Origin [x\|y\|z]"; no axis opens the dialog | axis x/y/z |
| brush.smooth / brush.hard (new) | same | new; sets the active brush's tip | – |
| sculpt.borderLock / sculpt.cornerLock (new) | same | new; bare toggles, `on`/`off`/`?` | – |

## Commands

| Operandi (old) | Map | Status | Arg renames |
|---|---|---|---|
| poly.flip / triple / quadruple / spinQuads / align / split / remove / merge / collapse / unify / make / setMaterial / setPart | same | already matches | poly.unify: `force:` accepted; poly.make: `type:` accepted; poly.setMaterial: `name:` accepted |
| mesh.cleanup | same | already matches (12 names match) | – |
| Cube / Sphere / Cylinder / Cone / Torus / Capsule Unit and … New Unit (new, layouts stage 3 batch 7) | – (not in the map) | Operandi's own, named like Ellipsoid Unit / Ellipsoid New Unit (LAY-Q42) | – |
| Mesh Paint, Mesh Paint Instance (new, missing-tools-g6) | – (not in the map) | Operandi's own phrases (a utility, like Retopo Sketch) | – |
| Contour (new, missing-tools-g4) | – (not in the map) | Operandi's own phrase and `tool.set Contour` id | – |
| view.topologyMode, Merge Instance (new, missing-tools-g3) | – (not in the map) | Operandi's own; `view.topologyMode` is the spec's name | – |
| Plane Item, Teapot, Teapot Item (new, missing-tools-g2) | – (not in the map) | Operandi's own phrases; the Teapot tool has no `tool.set` id yet | – |
| operandi.mesh.fixBlackDots, operandi.edge.zip, operandi.poly.diagonalRedirect (new, missing-tools-g1) | – (not in the map) | Operandi's own until the map has them (Fix Black Dots, Edge Zip, Diagonal Re-Direct) | – |
| @ppi_PolyScale_Offset.py, @ppi_flatten_ngons.py | same | already matches | – |
| `@ppi_PolyScale_Offset.py detach` | @ppi_PolyScale_Offset_Detach.py | **new name added**; the detach form stays as a deprecated alias | – |
| vert.set | same | already matches (axis, value, workPlane, worldSpace) | – |
| vert.new | same | already matches | **`x:` `y:` `z:` named args accepted** (positional still works) |
| vert.split / align / merge / join / collapse / remove / reorder / syncOrder | same | already matches; arg names match (align axis/uniform, merge range/dist/keep, join average, reorder mode) | – |
| vert.splitEdge | Vertex tab "Split Edge..." is `edge.split` | **no rename**: Operandi's `edge.split` is a different operation; renaming would collide. Kept | – |
| vert.removeEdge | Vertex tab "Remove Edge..." is `edge.remove` | **no rename**, same collision with Operandi's `edge.remove`. Kept | – |
| vert.weld target:<n> | no match | kept | – |
| **view.showVertices** | view3d.showVertices | **renamed**, old aliased | `state:` accepted |
| edge.subdivide / collapse / remove / split / join / growQuads / fillQuads / unbevel / splitNormals | same | already matches | edge.join: **`average:`** (and positional `1`) now picks Join Averaged |
| **edge.spin** | edge.spinQuads | **renamed**, old aliased | – |
| **edge.toCurves** | edges.toCurve | **renamed**, old aliased | – |
| **edge.joinAverage** | `edge.join [1]` | **alias** → `edge.join average:true` | – |
| edge.addLoop / edge.addPoint (one-shot) | same | already matches (`position`) | – |
| **selset.assign** | select.editSet | **renamed**, old aliased | name (positional) |
| **selset.use** | select.useSet | **renamed**, old aliased | name, mode (Operandi words replace/add/remove/select/deselect; the map's mode is an integer whose values aren't in the map: left alone) |
| **selset.delete** | select.deleteSet | **renamed**, old aliased | – |
| **selset.deleteAll** | `select.deleteSet all:true` | **alias** | `all:` accepted |
| **selset.group** | select.convertGroup | **renamed**, old aliased; Operandi's `<group> [set...]` arguments kept (the map's single `SSET` arg doesn't say how sets are listed) | – |
| selset.ungroup | no match | kept | – |
| preset.save, preset.load (new) | not in the map | Operandi names (Presets Stage 0); `kind:` item, assembly, environment, renderSettings, material; `path:` | – |
| **backdrop.create** | backdrop.add | **renamed**, old aliased | `name`, `image` (file path), `project` (front…camera, default front); named or in that order (BD-60). The map's other args (aspect, flip, autoSize, invert, blend, contrast, brightness, transparency, cenX, cenY …) aren't supported yet: use `backdrop.attr` |
| **backdrop.autoFit** | backdrop.fit ("Fit Backdrop Item to Geometry") | **renamed**, old aliased | – |
| clip.addStill (new, BD-60) | clip.addStill (per the coordinator's reading of the map; not in this repo's copy, to confirm) | loads a picture as an image entry only (UIC-165) | `filename` (named or positional) |
| backdrop.image, backdrop.attr, backdrop.size | backdrop.edit has a different shape (named options) | kept; see Left alone | – |
| item.add, item.pattern | not in the map; placeholders per BEHAVIOR-ITEM-LIST-HEADER.md | **already wired** on main (Add Item and the Select popover); help topics and MCP entries exist | – |
| tool.set / attr / reset / apply / drop | same | already matches | – |
| operandi.pen.create, operandi.sketch.sweep, pen, operandi.bench.* | no match (Operandi own) | kept | – |
| vertMap.new `type:uv` (UV editing stage 1) | vertMap.new | same command, new type: `uv` / `txuv` adds a UV map to the active mesh and selects it | name |
| operandi.uvMap.select / rename / delete / copy / paste, operandi.uv.fit / flip / mirror / align (new, uv-editing-2026-11-16) | no match | **placeholders**, see NAMES-NEEDED.md | name, newName; fit, keepProportion, gaps, gapsByPixel, udim; axis, perPolygon; center; direction, alignTo, perRow |
| operandi.statistics.select / hide / rename / delete / paste / clear / count (new, Statistics panel 2026-11-23) | no match | Operandi-only (UIC-165) | row, action, name, from |
| operandi.uv.preset (new, layouts stage 3), operandi.form.revert (new, layouts stage 3), operandi.notBuilt (new, layouts stage 3) | no match | **placeholders**, see NAMES-NEEDED.md | id, click; form; name |
| layout.window `hash:` palettes (layouts stage 6: uvEditor, uvView, toolbox, items, properties, shading, groups, vertexMaps, commandHistory beside presetBrowser) | layout.window | same command, more windows | hash, open |
| operandi.form.newGroup / new / add / addForm / addDivider / move / set / delete / select / find / preview / export (new, layouts stage 7) | no match | **placeholders**, see NAMES-NEEDED.md; all class "ui" | name; id, label, style, group; line, sub, index, form; from, to; attr, value; on; filename |
| operandi.scene.current / project.* / inputMap / select.polygonTagType / lasso.* / view.showLocked / view.showFalloffs / viewport.close / help.navigation / window.* / mcp.bridge (new, menus as forms) | no match | **placeholders**, see NAMES-NEEDED.md | index; path, name; preset; type; style, on; size |
| viewport.split / viewport.detach (new, menus as forms) | viewport.split / viewport.detach | **match** (their arguments aren't read yet) | – |
| operandi.layout.togglePalettes (new, layouts stage 6) | no match | **placeholder**, see NAMES-NEEDED.md; class "ui" | – |
| operandi.layout.setKey / clearKeys (new, layouts stage 5); operandi.layout.revert `keys:` | no match | **placeholders**, see NAMES-NEEDED.md; class "ui" | action, key, layout; layout; keys |
| operandi.workbench.edit / pick / add / addProperty / addDivider / addPipe / move / set / chooseIcon / duplicate / delete / undo / topBar (new, layouts stage 4) | no match | **placeholders**, see NAMES-NEEDED.md; all class "ui" | on, form; line, label; property; as; from, to; index, attr, value |
| operandi.view3d.matcap | no match (Operandi own: the Matcap style's matcap choice, 2026-11-10) | kept | name, image, bgmesh |
| view3d.shadingStyle value Reflection | shown as Matcap (VS-N01, Jason 2026-11-10) | Reflection still accepted | value |
| poly.subdivide (new) | same | **new**, map name | `type` (positional or `type:`; `ccsds`, `flat`, `smooth`, `sds`), `fractal` (fraction), `maxangle` (radians), `linear`, `bound`, `bgCons`, `noEdgeWeight`; bare `poly.subdivide` opens the dialog (SUBT-Q01) |
| poly.unsubdiv (new) | same | **new**, map name | `iteration`, `flat`, `uv`, `material`, `part`, `smoothingGroup`, `selectionSet`, `selectionSetName` |
| vert.center (new) | same | **new**, map name | `axis` (positional or `axis:`): x, y, z, xy, yz, zx, all |
| poly.freeze (new) | same | **new**, map name; opens the Freeze dialog. The old `freeze` phrase still runs Freeze directly | map options (`face`, `drill`, `disp`...) not built |
| MCP only: **poly.flattenNgons** | @ppi_flatten_ngons.py | **renamed**, old aliased | – |
| MCP only: mesh.new, item.delete/rename/visibility/parent/drop, symmetry.set | no toolbar row | kept | – |
| **item.select** | select.item (Select Item) | **renamed**, old aliased (core registry, typed and MCP; planar-drag 2026-11-09, PD-Q12) | item; `mode` and `mask` accepted, not used yet |
| transform.channel (new) | transform.channel (Transform Channel) | **new** in core's registry, typed and MCP; value `?` reads | name (pos.X/Y/Z, rot.X/Y/Z, scl.X/Y/Z, provisional), value, item; map's `mode`, `filter`, `replace` refused for now |
| transform.reset (new) | named by spec freeze-transforms-2026-11-14 §8 | **new** in core's registry, typed and MCP; Item Properties Reset and Item > Transform run it | `type` (positional or `type:`): translation, rotation, scale, all (default); shear reserved |
| transform.freeze (new) | named by spec freeze-transforms-2026-11-14 §8 | **new**, as transform.reset; not the geometry `poly.freeze` / `mesh.freeze` / `deformer.freeze` | `type`, as transform.reset |
| item.transform | no counterpart | Operandi-only (registry-2), typed and MCP; on the live item tool's Texture Locator it takes transform.channel's in-tool route | item, translation, rotation, scale |
| MCP only: select.symmetrical | same | already matches | – |

## Units (Jason, 2026-11-07: scripts use the map's units)

Command and script arguments take the map's units; panels keep showing %
and degrees. The conversion happens where arguments are parsed
(`command_names::script_percent` / `script_angle`), not in the tools.

- **percent** arguments: a bare number is a fraction (0.5 = 50 %); a value
  with `%` is a percent (`50%`). Converted: `chamfer.edit depth`,
  `tool.attr edge.bevel|edge.chamfer depth`, `tool.attr poly.reduct
  boundaryWeight|materialWeight`, `edge.addLoop|edge.addPoint position`, MCP
  `edge.subdivide percent`. Already 0–1 before: `tool.attr poly.smshift
  scale`, `tool.attr edge.bridge tension`.
- **angle** arguments: a bare number is radians; `30deg`, `30°` or `0.5rad`
  keep their unit. Converted: `tool.attr poly.smshift maxAngle`.
- Old-unit values: a bare `depth:50` now means 5000 % and is refused where
  the tool has a range (Add Loop/Point position), so old scripts fail loudly
  rather than silently. Writing `50%` works in both. Nothing saved stores
  these arguments: the input map keeps action labels, pie entries keep
  Operandi phrases (`View Top`), scenes keep tool state, so no migration is
  needed.
- `?` queries still answer with a unit (`50 %`, `45 deg`), which reads back
  under the same rules.
- Left alone (not map arguments): the legacy phrases `smooth shift angle
  30` / `smooth shift scale 50` (Operandi's own, degrees and %), MCP
  `tool_run` JSON fields (`smooth_shift.angle` degrees, `chamfer.depth`
  percent, `rotation_delta` degrees), `backdrop.attr` contrast/brightness/
  transparency (no map counterpart; -100..100 %).
- `vert.set axis`, `vert.align axis`: the map's type is integer; Operandi
  takes the index (0/1/2) or x/y/z, so both work.
- `edge.bevel mode`, `edge.chamfer edgeEnd`, `select.useSet mode`: the map
  types are integers; Operandi takes words. Integer values aren't in the map.

## Counts

- Renamed (old name now an alias): 20 (10 tools incl. `tweak`, 10 commands).
- Aliases with fixed args: 3 (`edge.joinAverage`, `selset.deleteAll`,
  `@ppi_PolyScale_Offset.py detach`).
- New map names added on top of existing behaviour: 3 (`*.Slide`,
  `@ppi_vert_tear.py`, `@ppi_PolyScale_Offset_Detach.py`).
- Arg renames: 4 (`sourceSegments`, `useMaterial`, `material`, `legacy`);
  new named args accepted: 7 (`x/y/z`, `state`, `force`, `type`, `name`,
  `average`, `all`).
- Arguments converted to map units: 9 (8 percent, 1 angle).
- Already matching: 70+ names. No match (kept): 19.

## GUI check (one pass, Jason or Codex)

1. Command line: `tool.set tweak on`, then `tool.set ElementMove ?` shows 1.
2. `tool.set prim.torus on` and `tool.set prim.toroid on` both start Torus.
3. Vertex mode, `tool.set "*.Slide" on` starts Vertex Slide; Edge mode, Edge Slide.
4. `selset.assign A` then `select.useSet A` restores the selection.
5. Polygon panel tooltips and Help text show the new names (e.g. Merge tool
   tooltip says `tool.set vert.merge on`).
6. Toolbox labels and shortcuts are unchanged (Element Move stays T).

## Mesh Stack v1 (registry thread, MESHOPS-NAMES rev 2, 2026-11-11)

| Name | Operandi | Notes |
|---|---|---|
| meshop.create type:<op type> | Add Operator | inserts above the selected row; mode/category/applyDefaultPreset accepted, unused |
| item.delete item:op:<item>:<op> | Delete op | ops aren't items in Operandi (MOQ-07) |
| item.parent item:op:… position:<n> | Move op | position in evaluation order |
| deformer.enable deformer:… enable:<bool> | op eye | |
| item.channel <attr> <value> item:op:… | op values | reference attribute names |
| select.item op:… / op:<item>:base | list row | |
| deformer.freeze duplicate:<bool> deformer:… | Freeze / Duplicate and Freeze | |
| mesh.freeze | Freeze whole stack | |
| deformer.selectBaseMesh item select:<bool> | Edit Base Mesh | |
| selop.indexcreate, selop.selection mode:set\|clear\|add\|remove | Select by Index | |

Op types (saved): prim.cube.item, prim.sphere.item, prim.cylinder.item,
poly.bevel.item, poly.smshift.item, subdivide.tool.item,
pmodel.triangulate.item, mirror.clone.item, array.clone.item;
Select by Index index.selop.item. One mapping: `mesh_stack::OP_TYPES`.

## Curve tab commands (2026-11-17)

`poly.makeCurveOpen`, `poly.makeCurveClosed`, `poly.makeCurveFill`,
`curve.winding merge:<bool> reverse:<bool>` and `curves.setColor
override:<bool> color:"r g b"` take the names the curve commands spec
lists (CURVE_COMMANDS_ACCEPTANCE). The 2D shapes are Mesh Stack
(`meshop.create type:curve.circle.item`, `curve.ellipse.item`,
`curve.rectangle.item`, `curve.diamond.item`, `curve.nsided.item`,
`curve.star.item`).

## 3D painting stage 1 (painting builds only)

Operandi's own names (PNT-Q01; the map has none): `operandi.image.new`,
`.newSequence` (greyed), `.invert`, `.clear`, `.setResolution`,
`.expandSeams`, `.flatten`, `.toVectorDisplacement` (greyed), `.save`,
`.saveAs`, `.saveAll`, `.reload`, `.select`; `operandi.paint.addTexture
kind:color|mask|displacement`, `.addFurTexture` (greyed), `.target`,
`.images`, `.psdLayer`, `.newPsdLayer`, `.alsoSaveImages`, `.autoCreate`,
`.uvBorder`. Not in a default build's registry
(`no_painting_tests.rs`).

Stage 2 (painting builds only): `operandi.paint.airbrush`, `.paintbrush`,
`.eraser` (tools), `.tip kind:smooth|hard`, `.set <name>:<value>` (every
paint tool row), `.notYet name:<tool>` (greyed later-stage buttons).

Stage 3 (painting builds only): `operandi.paint.smudge`, `.blur`,
`.sharpen`, `.clone`, `.line`, `.fill`, `.linearGradient`,
`.radialGradient` (tools), `.lasso`, `.lassoSet`, `.lassoClear`; `.set`
gains `fillTolerance` and `lineShape`.

Stage 4 (painting builds only): `operandi.paint.tip` takes procedural,
sphere, image, stamp, text; `operandi.paint.tipSet` (tip and nozzle rows),
`.nozzle`, `.savePreset`, `.applyPreset`.

Stage 5 (painting builds only): `operandi.paint.ink kind:<image|parametric|random|projection>`
(adds or removes an ink), `.inkSet kind:<ink> <row>:<value>` (ink rows),
`.inkWidget action:<show|hide|turnRight|turnLeft|smaller|larger>` (Image
Ink's widget; E/R/D/F), `.sculptInks` (greyed); `operandi.paint.set` gains
`curveConstraint` (none, background2D, background3D).

Stage 6 (painting builds only): `operandi.paint.layout` (the Paint layout's
open command), `operandi.paint.unavailable name:<command>` (greyed sculpt
commands), `operandi.image.view image:<id> layer:<name|composite>` (Image
Viewer), `operandi.paint.lassoEdit moveX moveY aboutX aboutY scale` (the
lasso's handles); `pref.value painting.antialiasing | newImageResolution |
imagesAsIcons | autoCreate | defaultForeground | maxBrushSize |
defaultBrush`.

Stage 7 (painting builds only): `operandi.paint.sculptTip kind:<…>`,
`.sculptTipSet` (the sculpt tip's values), `.maskBrush kind:<paint|smooth|
scaleDown|erase|setValue>`, `.maskSet enable invert opacity`, `.maskClear`,
`.maskBakeFalloff`, `.applyStarter name:<starter>`, `.geometryToBrush mode
resolution axis iterations`.
