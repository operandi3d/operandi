# MCP bridge

Status: first slice built (2026-11-03). Crate `crates/operandi-mcp`.
Built into the app (Window > MCP Bridge) on 2026-11-03.

The bridge lets an agent drive Operandi headlessly over the Model Context
Protocol: read scene and mesh state, change the selection, run tools and
one-shot commands with parameters, and undo or redo. The research side can
then run one recipe in each application and compare results. Nothing in this
repo talks to Modo or compares against it; the comparison happens on the
research side.

## Shape

```
agent (MCP client)
   │  JSON-RPC 2.0, one message per line
   ▼
operandi-mcp  (sidecar binary)
   protocol.rs   initialize / ping / tools/list / tools/call
   bridge.rs     MCP tool → operandi_core::Command, scene → JSON
   ▼
operandi_core::Editor   (same dispatch, history and validation as the app)
```

Think of `Editor` as the hypervisor and the GUI as one management console:
the bridge is a second console on the same control plane, not a separate
modeling implementation. Every edit goes through `Editor::dispatch(Command)`,
so selection rules, symmetry, falloff, history labels and refusals are the
app's own.

### Transport

The MCP **stdio** transport: the client launches `operandi-mcp`, writes one
JSON-RPC message per line on stdin, and reads responses from stdout.
Protocol revisions 2025-06-18, 2025-03-26 and 2024-11-05 are accepted;
an unknown request gets the newest. No async runtime and no new third-party
dependencies (only `serde_json` and `glam`, already in the workspace).

```bash
cargo build -p operandi-mcp
# Claude Code / any MCP client:
claude mcp add operandi -- /path/to/target/debug/operandi-mcp
# Batch mode, no client: run a recipe and print one JSON line per step.
target/debug/operandi-mcp --recipe recipe.json
```

A recipe is a JSON array of `{"tool": "<mcp tool>", "arguments": {...}}`.
It stops at the first failing step and exits 1.

### Inside the running app

The app answers the same messages against the scene on screen. Two ways in:

| Launch command | Scene it drives |
|---|---|
| `operandi-mcp` | its own headless scene (no window, no GPU) |
| `operandi-mcp --attach [SOCKET]` | the running app's scene, via Window > MCP Bridge |

```bash
claude mcp add operandi-live -- /path/to/target/debug/operandi-mcp --attach
```

- **Default:** on in development/QA builds (including `scripts/bundle-dev.sh`
  bundles), off in packaged release builds (`OPERANDI_MILESTONE1_PACKAGE=1`).
  `OPERANDI_MCP=1` or `OPERANDI_MCP=0` at launch overrides either. An
  always-open control socket is fine on a dev machine and not something to
  ship switched on.
- **Toggle:** Window > MCP Bridge (checkmark). While on, the window title ends
  in `· MCP` and the status bar names the socket.
- **Socket:** a Unix domain socket, owner-only (0600), at
  `$TMPDIR/operandi-mcp.sock` or `OPERANDI_MCP_SOCKET`. Not a network port:
  nothing off the machine, and no other user on it, can connect. A second
  Operandi leaves a live socket alone; a stale one from a crash is replaced.
  macOS and Linux only for now.
- **Threads:** socket threads only move JSON lines. Each request runs on the
  UI thread between frames (`crates/operandi-app/src/mcp.rs`), against the
  app's own `Editor`, so undo labels, the viewport and the person's Undo all
  see the agent's steps.
- **The person at the keyboard wins.** Editing calls are refused (as
  `isError` results) while a dialog is open, a drag is in progress, or a tool
  is active in the app; the refusal names the dialog, and `scene_info`
  reports it as `dialog` (null when none). No bridge call opens a dialog.
  `scene_new` / `scene_load` are refused while the
  document has unsaved changes, and go through the app's own New/Open paths
  so app-wide settings, the saved copy and the title stay right.
  `scene_load` opens `.opscene` only. Reads (`scene_info`, `mesh_get`) and
  `scene_save` always run. `scene_save` is Save As (the document takes
  that path and its title loses the `*`), so `scene_new` works after it;
  `copy: true` saves a copy and leaves the document alone. `scene_revert`
  reopens the document's file, dropping unsaved changes. `scene_load` of
  the file that is already open reads it from disk again (so an edit made
  to the file shows); a file open in another scene tab is switched to and
  read again, unless that tab has unsaved changes.
- **Later answers.** `viewport_capture` and `render` are answered on a
  later frame: the request waits on its socket thread while the app keeps
  drawing, so the bridge never blocks the window. A capture gives up after
  10 s (no frame drawn: window hidden); a render answers when done or when
  its `timeout` (default 120 s) runs out, saying it is still rendering.

## Command surface

| MCP tool | What it does | Core mapping |
|---|---|---|
| `scene_info` | items (id, name, parent, visible, transform, counts), selection, live tool, symmetry, undo/redo labels, `document` {path, name, modified} | reads `Editor` (app: its document) |
| `mesh_get` | vertices, polygons (vertex loop, material, part), edges, the smoothing rule; item or world space. Faces no longer carry `smooth` (it never changed shading); shading comes from `smoothing`, set with `command_run smoothing.set` | reads `Scene` |
| `scene_new` | fresh scene with one empty mesh, no history | `Editor::default` + `NewMesh` |
| `scene_load` / `scene_save` | `.opscene` files (`scene_save {path, copy?}`) | `persistence::load`, `Editor::save_committed` (app: File > Save As) |
| `scene_revert` | app only: reopen the document's file, dropping unsaved changes | app's open path |
| `scene_import` | add an OBJ, STL, glTF/GLB, GEO, PLY, FBX, Alembic, 3MF or USD file's meshes to the open scene (`{"path": "/x/vase.obj"}`): same reader and File I/O preferences as File > Import, but the scene is kept (File > Import replaces it); the meshes come in as separate items with no hierarchy and their transforms baked into the points, except USD (.usd/.usda/.usdc/.usdz), whose Xform hierarchy arrives as parented items with local transforms (the import dialog's defaults; docs/BEHAVIOR-USD-IMPORT.md); the file's material groups join when the scene has none for their tags, the new items become the selection; one undo step "Import <file>"; answers `imported` [{id, name, parent}] and the reader's `notes`. Never opens a dialog: a missing, unreadable or unsupported file (`.lxo`, `.opscene`) is an error and nothing changes | `interchange::import` → `LoadPreset` (Assembly) |
| `select` | mode plus vertices/edges/faces by index; replace or add; `item` makes that item active in any mode; one "Selection" undo step | `Mode`, `SelectItem`, `BeginSelection`, `ClearSelection`, `Select`, `EndSelection` |
| `selection_op` | grow, shrink, loop, ring, all, invert, connected, boundary, more, less, between, close_loop, convert | `Selection(SelectionOperation)` |
| `tool_run` | activate a tool, set parameters, commit; one undo step (Element Move: `{"tool": "ElementMove", "params": {"tweak": {"vertex": 3}, "delta": [x, y, z]}}`, also `edge: [a, b]` or `polygon: n`; optional `"tweak_options": {"mode": "Auto Center", "range": 1.5, "connect": "Use Connectivity", "shape": "Smooth"}`) | `Activate` → `Preview` (or `ConfigureSmoothShift`) → `Apply` for primitives and Smooth Shift/Thicken → `Drop` |
| `command_run` | one-shot commands (poly.*, edge.*, vert.*, item.*, symmetry.set, select.symmetrical, smoothing.set); an argument the command doesn't take is an error | the matching `Command` variant |
| `command_help` | every command's arguments: name, type, unit, default (below) | `crates/operandi-mcp/src/commands.rs` |
| `primitive_add` | Cube, Sphere, Cylinder, Cone, Ellipsoid, Torus, Capsule, Arc or Tube (any case) on the active mesh from the tool's Reset values plus the given settings; `position` is the centre in **world space** (below), e.g. `{"shape": "torus", "params": {"ring_radius": 1, "sides": 32}}` or `{"shape": "tube", "params": {"points": [[0,0,0],[0,1,0]]}}`; one undo step | Cube/Sphere/Cylinder: `Activate` → `Preview` → `Apply` → `Drop`; others `ArmCone`/`ArmEllipsoid`/`ArmShape` → `Configure…` → `Apply` → `Drop` |
| `item_get` | one item's kind, transform and own properties (camera lens, film, clipping, depth of field, resolution; light type, intensity, colour, shape; a mesh's subdivision levels and polygon kinds), each with its unit | reads `Scene` |
| `shader_tree_get` | the shader tree top row first (id, parent, depth, name, type, effect, eye, group tag); with `layer`, one layer in full: Layer block, channels with units, Gradient input and keys, Vertex Map map, Image Map image | `Scene::render_shader_tree` |
| `material_get` / `material_set` | one Material's channels by material tag (`poly.setMaterial` name, default "Default"), Material layer name or `layer` id, e.g. `{"material": "Red", "diffuse_color": [0.8, 0.05, 0.05], "roughness": 0.25, "metallic": 1}`; also shading_model, name, enable, opacity, blend; one undo step | `ShaderTreeEdit::Replace` (the Shading panel's edit) |
| `shader_layer_add` / `shader_layer_set` | add a layer (material, group, folder, shader, constant, image_map, gradient, vertex_map, variation, process, occlusion, or a procedural) into `parent` or a `material`'s group, then set it; or change one layer: effect, Gradient `input` and `value_keys` / `color_keys`, Vertex Map `map`, Image Map `image` (PNG/JPEG path or scene image id), group `tag`, `channels`; one undo step | `ShaderTreeEdit::Add`, `Replace`, `SetImage` |
| `subdivision_set` | Tab / Shift+Tab (`toggle`: subdivision or catmull_clark) on the selected polygons (all when none, or outside Polygon mode) and the mesh's levels and boundary rules; one undo step | `ToggleSubdivision`, `SetSubdivision` |
| `camera_set` / `light_set` | camera or light properties, one undo step | `EditCamera` / `EditLight` (the panels' edits), `SetRenderCamera` |
| `view_set` | app only: a view shows a named view or looks through a camera, and its `shading` style (wireframe, solid, vertex_map, shaded, texture, default, advanced, gooch, cel, reflection; the Display menu's names work too); `viewport` (index, 0 = first) picks the view and makes it active, default the active view; answers view, shading, viewport and the viewport count. Not saved with the scene, not an undo step | Camera view; `view3d.shadingStyle` |
| `panel_show` | app only: bring a side panel's tab to the front (`items`, `shader_tree`, `groups`, `properties`), adding a closed one back to its group, so a capture or screenshot shows it; answers both groups' tabs and front tab. Layout only: runs with a dialog open, opens none | panel groups (the tab strip's click) |
| `viewport_capture` | app only: PNG of the active view (current, a named view, or through a camera), saved and returned as an image | eframe screenshot, cropped to the view |
| `render_settings_get` / `render_settings_set` | render settings: samples, denoise, adaptive threshold (scene, saved, native version 27), render camera resolution, and the Final Color output's exposure, white level, ISO and gamma; one undo step | `SetRenderSettings`, `EditCamera`, `ShaderTreeEdit::SetOutput` |
| `render` / `render_status` | app only: Cycles render to a file at a chosen size and sample count; the answer returns the picture as an image (`include_image`, default true; `max_size`, default 1024 px longest side), as `viewport_capture` does; `render_status` gives progress and the last result, and its picture with `include_image: true` | Render > Render Viewport |
| `undo` / `redo` | `count` steps; refuses more than the stack holds | `Undo` / `Redo` |

Tool ids reuse the app command line's spellings (`prim.cube`, `poly.extrude`,
`TransformMove`, `edge.slide`, `vert.bevel`...), so a recipe reads like the
command line. `tool_run` parameters are the fields of
`operandi_core::Parameters` (Move `delta` is an offset, Rotate `delta` is
radians, Scale `delta` is factors; Extrude uses `extent`, Bevel `inset` and
`shift`). An unknown parameter or argument is an error rather than being
ignored, so a recipe can't silently run with defaults.

`tool_run CurveExtrude` extrudes the selected polygons along a path, one
undo step, from the Curve Extrude button's Reset values (Start at Source
on, Cap Start off, Invert on). Its options go in `params.curve_extrude`:
`points` (`[[x, y, z], ...]`, world metres, in order; required), `closed`,
`steps` (default 24), `kind` (`curve`, `bezier`, `b_spline`, `pen`),
`start_control`, `end_control`, `by_length`, `create_curve_polygon`,
`bank` and `twist` (radians), `automatic`, `smoothing_angle` (radians),
`align_to_path`, `start_at_source`, `cap_start`, `cap_end`, `invert`,
`size` (the last section's scale) and `corner_scale` (pen paths). Example:
`{"tool": "CurveExtrude", "params": {"curve_extrude": {"points": [[0, 0, 0],
[0, 1, 0], [1, 2, 0]], "steps": 4}}}`.

Every edit returns a short state echo: revision, active mesh counts,
selection (the mode and every mode's own list, in pick order: each mode
keeps its list while another is current), undo and redo depth.

### Units

One rule for every tool and argument: **metres, radians, fractions 0–1**
(1 = 100 %), pixels for picture sizes, kelvins for colour temperature. The
panels show millimetres, degrees and percent for the same values; the
bridge converts where a tool keeps those (Rotate, Transform, Smooth Shift
angle, Arc angle, Tube twist, Chamfer depth, Smoothing). Camera film and
focal length are metres too: `focal_length 0.05` is 50 mm. Shader channels
are named after the Shading panel's rows in snake_case ("Clearcoat Amount"
is `clearcoat_amount`); angles there (Smoothing Angle, Gradient keys on an
angle input) are radians too. `light_set render` takes the panel's Default,
Yes and No (On and Off mean Yes and No).

### Primitive sizes

| Shape | Size setting | Meaning |
|---|---|---|
| cube (`prim.cube`, `primitive_add cube`) | `size [x, y, z]` | **full** edge lengths |
| sphere, cylinder, ellipsoid, capsule | `radius [x, y, z]` or one number | **half** extents (cylinder Y = half the height) |
| cone | `size` or `radius` `[x, y, z]` | **half** extents |
| torus | `ring_radius` | axis to the middle of the tube |
| torus | `cross_section`, `height` | **half** extents of the tube: across the ring (outer radius = ring_radius + cross_section) and along the axis (the torus is 2 × height tall) |
| arc, tube | `radius` (a number) | as named |

**Position.** `position` is the primitive's centre in world space, metres,
for cube, sphere, cylinder, cone (centre of its bounding box), ellipsoid,
torus and capsule; without it the centre is the world origin. The geometry
is built in world space and then put into the active mesh's item space, so
a moved, turned or scaled mesh item still gets the primitive at that world
point (its vertices in `mesh_get` without `world` are item space). The work
plane and action centre are not used. Arc (`center`, `start`, `end`) and
tube (`points`) take world-space points instead.

`sides` is always the count around the axis. `segments` is the count along
the axis: latitude bands for the sphere, rows for the cylinder; the cube's
is `[x, y, z]` (or one number). `tool_run prim.sphere` names it
`radial_segments`. `tool_run prim.*` starts from the segment counts last
used in the app; `primitive_add` always starts from the Reset values.

### Command arguments

`command_help` returns this list; a test keeps it the same as here.

<!-- command-args -->
- **`poly.setPart`** — Set the Part tag of the selected polygons.
  - `name` (string, required): Part name.
- **`uv.unwrap`** — UV Unwrap of the visible polygons into the UV map (one-shot for now); unnamed attributes keep the last values.
  - `iter` (integer): 0 keeps the Initial Projection (default 100).
  - `gaps` (number): Relative Gap Size, 0-1 (default 0.2).
  - `matr` (boolean): Segment by Material.
  - `part` (boolean): Segment by Part.
  - `project` (integer|string): 0 planar, 1 cylindrical, 2 spherical, 3 current, 4 groupNormal.
  - `pinning` (integer|string): 0 u, 1 v.
  - `axis` (integer|string): Projection Axis: 0 x, 1 y, 2 z.
  - `angle` (number): Offset Angle in radians.
  - `seal` (boolean): Seal Holes.
  - `layout` (boolean): Update Layout.
  - `mode` (integer|string): 0 conformal, 1 angleBased.
  - `symmetrize` (boolean): Symmetrize (not built yet; must be false).
  - `symmAxis` (integer|string): 0 x, 1 y, 2 z.
  - `symmComp` (integer|string): Mirror Axis: 0 u, 1 v.
  - `name` (string): UV map; made when missing (default Texture).
  - `seam` (integer|string): 0 selectedEdges, 1 seamMap, 2 selectionSet.
  - `seamName` (string): Seam map name.
  - `pickName` (string): Edge selection set name.
- **`uv.relax`** — UV Relax of the active mesh's UV map on its visible polygons, one undo step. A polygon selection doesn't limit it; with noEdge false, islands holding a selected edge stay still. Unnamed attributes take the tool's Reset values; iter 0 (the Reset value) changes nothing.
  - `mode` (integer|string): 0 spring, 1 conformal (default), 2 angleBased, 3 adaptive.
  - `iter` (integer): Iterations, the amount of relaxing (default 0: no change).
  - `seal` (boolean): Seal Holes (Conformal and Angle Based only; default false).
  - `boundary` (integer|string): 0 smooth (default), 1 locked, 2 maintain, 3 straighten; not used in Spring mode.
  - `area` (number): Area Weight, 0-1 (Adaptive only; default 0.1).
  - `uniform` (boolean): Uniform Spans (Straighten only; default false).
  - `noEdge` (boolean): Ignore Edge Selection (default true); false holds islands with a selected edge still.
  - `name` (string): UV map (default: the selected one).
- **`operandi.uv.atlas`** — Atlas Unwrap of the active mesh's visible polygons into the UV map: charts of neighbouring polygons whose normals stay within angle of the chart's first, packed into 0-1 at one texel density. One undo step. Unnamed attributes keep the last values.
  - `name` (string): UV map; made when missing (default Texture).
  - `angle` (number): Most a chart bends, radians (default 1.0472, 60°).
  - `gaps` (number): Space around each chart in texels at resolution (default 4).
  - `resolution` (integer): Texture size the gaps are measured at (default 1024).
- **`uv.pack`** — Pack UVs of the UV map; only the selected polygons' islands move when polygons are selected.
  - `pack` (boolean): Islands may move.
  - `stretch` (boolean): Match each island's UV area to its 3D area first.
  - `orient` (boolean): Islands may rotate.
  - `keepSize` (boolean): Islands keep their size.
  - `direction` (integer|string): 0 horizontal, 1 vertical, 2 auto.
  - `gaps` (number): Gaps, 0-1 (default 0.2).
  - `byPixel` (boolean): Set Gaps by Pixel.
  - `gapsByPixel` (number): Pixel Size (gap in pixels of a 1024 texture).
  - `bbox` (boolean): Based on Bounding Box.
  - `stack` (boolean): Lock Stacked.
  - `region` (integer|string): 0 normalized, 1 nearest, 2 udim, 3 manual, 4 tiles, 5 polygonTag, 6 byItem.
  - `udim` (integer): Target UDIM tile (from 1001).
  - `regionX` (number): Manual region corner U.
  - `regionY` (number): Manual region corner V.
  - `regionW` (number): Manual region width.
  - `regionH` (number): Manual region height.
  - `tileU` (integer): Tiles wide.
  - `tileV` (integer): Tiles high.
  - `polygonTag` (integer|string): 0 material, 1 part, 2 smoothingGroup, 3 selectionSet: the Polygon Tag region's groups.
  - `background` (boolean): Background Constraint: other visible meshes' islands stay put and are packed round.
  - `writeNew` (boolean): Pack UVs to New Map: keep the original map.
  - `newName` (string): The new map's name (with writeNew).
  - `bestrot` (boolean): Find Best Box Rotation.
- **`operandi.uvMap.select`** — Select a UV map of the active mesh (the UV view, the UV tools and Unwrap use it), one undo step.
  - `name` (string, required): The map's name.
- **`operandi.uvMap.rename`** — Rename a UV map of the active mesh, one undo step.
  - `name` (string, required): The map's name now.
  - `newName` (string, required): Its new name.
- **`operandi.uvMap.delete`** — Delete one or several UV maps of the active mesh in one undo step.
  - `name` (string, required): The map's name, or several separated by commas.
- **`operandi.uvMap.copy`** — Copy a whole UV map's values to the UV map clipboard (no undo step).
  - `name` (string, required): The map's name.
- **`operandi.uvMap.paste`** — Replace a UV map's values with the copied map's (same polygons needed), one undo step.
  - `name` (string, required): The map pasted into.
- **`operandi.uv.fit`** — Fit UVs: scale and move the selected UVs (all visible ones with nothing selected) to fill a UDIM tile, one undo step. Unnamed attributes keep the last values.
  - `fit` (string): entire (default), island or polygon.
  - `keepProportion` (boolean): Scale uniformly (default true).
  - `gaps` (number): Margin inside the tile, percent (default 0).
  - `gapsByPixel` (number): The margin in pixels of a 1024 texture instead.
  - `udim` (integer): The tile (default 1001).
- **`operandi.uv.flip`** — Flip U or V: mirror the selected UVs about their bounding-box centre, or each polygon about its own, one undo step.
  - `axis` (string): u (default, left to right) or v (top to bottom).
  - `perPolygon` (boolean): Each polygon about its own centre.
- **`operandi.uv.quickAlign`** — Quick Align (UVT-54): turn each island holding the selection so its first selected edge (or the line between its first two selected points) lies along an axis, one undo step.
  - `mode` (string): auto (default: the nearer axis), horizontal or vertical.
- **`operandi.uv.islandAlign`** — Island Align (UVT-55): move each selected island so their left sides, right sides or centres (u) and bottoms, tops or middles (v) line up, one undo step.
  - `u` (string): min, max, center or unchanged (default).
  - `v` (string): min, max, center or unchanged (default).
- **`operandi.uv.orient`** — Orient Pieces (UVT-56): turn each island in its smallest box (auto), or with its longer side along U (horizontal) or V (vertical), one undo step.
  - `mode` (string): auto (default), horizontal or vertical.
- **`operandi.uv.toOrigin`** — To Origin (UVT-57): move each selected island so its lower-left corner sits on the tile's lower-left corner, one undo step.
  - `udim` (integer): The tile (default 1001).
- **`operandi.uv.texel`** — Texel Density (UVT-58/59): density = Image Size x sqrt(UV area / 3D area). Apply individually or overall, or normalize, one undo step.
  - `action` (string): individually (default), overall or normalize.
  - `imageSize` (number): Texture size in pixels (the panel's value).
  - `density` (number): Target pixels per metre (the panel's value).
  - `worldSpace` (boolean): Item scale counts (the panel's value).
  - `upright` (boolean): Turn islands upright first (Alt).
- **`operandi.uv.realWorldSize`** — Real World Size: scale the selected islands together so one UV unit covers size metres, one undo step.
  - `size` (number): Metres one UV unit stands for (default 1).
  - `worldSpace` (boolean): Item scale counts (default true).
- **`operandi.uv.split`** — Split UVs (UVT-60): split along the selected edges, round a polygon selection, or at selected points, the sides moved apart by distance, one undo step.
  - `size` (string): plain (default, invisibly apart), medium or large.
  - `distance` (number): A custom distance in UV units instead.
- **`operandi.uv.sew`** — Sew UVs (UVT-61): join each selected seam's two sides into one UV position, one undo step.
  - `connect` (string): average (default), selected or unselected.
  - `selectedOnly` (boolean): Keep the unselected side still (Move Selected Only).
- **`operandi.uv.moveSew`** — Move and Sew (UVT-62): move (turn, and scale) the island across each selected seam so the seam lines up, then join it, one undo step.
  - `connect` (string): average (default), selected or unselected.
  - `scale` (boolean): Scale the island to fit (default true).
- **`operandi.uv.selectBorder`** — Select UV Border (UVT-80): select the edges on every island's outline. No arguments.
- **`operandi.uv.selectErrors`** — Select Error UVs (UVT-81): select the polygons that fail any ticked test.
  - `cross` (boolean): Overlapping another UV polygon.
  - `self` (boolean): Twisted (bow-tie) UV polygons.
  - `flipped` (boolean): Wound clockwise in UV.
  - `stacked` (boolean): Exactly on top of another.
  - `udim` (boolean): Crossing a UDIM tile border.
  - `zero` (boolean): Zero UV area.
- **`operandi.uv.selectUnmapped`** — Select Unmapped (UVT-82): select the polygons with no UVs in the selected map, or the mapped ones with invert.
  - `invert` (boolean): Select the mapped polygons instead.
- **`operandi.uv.copy`** — Copy UVs (UVT-90): the selected polygons' UVs onto the UV clipboard. No arguments.
- **`operandi.uv.cut`** — Cut UVs (UVT-90): copy, then remove the selected polygons' UVs, one undo step. No arguments.
- **`operandi.uv.paste`** — Paste UVs (UVT-91): write the clipboard onto the selected polygons, matched by topology or by selection order, one undo step.
  - `mode` (string): connectivity (default) or selection.
- **`operandi.uv.delete`** — Delete UVs (UVT-93): the selected polygons become unmapped, one undo step. No arguments.
- **`operandi.uv.setValue`** — Set UV Value (UVT-94): set U, V or both of the selected UVs, one undo step.
  - `u` (number): The U value.
  - `v` (number): The V value.
- **`operandi.uv.rotatePolygons`** — Rotate Polygon UVs (UVT-95): each selected polygon's corners take the next corner's UV, one undo step. No arguments.
- **`operandi.uv.mergeMaps`** — Merge UV Maps (UVT-96): fill the first map from the others where it has no UVs, then delete the others, one undo step.
  - `names` (string): The maps, comma separated, the target first.
- **`operandi.uv.seam`** — UV seam maps (UVT-100/101, UVS-1 §1.6): add the selected edges (all with none selected), clear them, select the map's edges, delete the map, or mark the Conditional Loop from the selected edges, one undo step.
  - `action` (string): add (default), clear, select, delete, marker, or use (pick the map the buttons use).
  - `name` (string): The seam map (default: the one in use, UV Seam to start).
- **`operandi.uv.symmetry`** — The UV Symmetry layer (UVT-70): in a UV view, an edit to a UV is copied, mirrored, to its partner across the line.
  - `axis` (string): off (default), u or v.
  - `center` (number): The line's position (default 0.5, the last used).
- **`operandi.uv.symmetryCenter`** — Center (UVT-70): UVs on the UV symmetry line land on it, and one point's UVs there become one, one undo step. No arguments.
- **`operandi.uv.symmetrize`** — Symmetrize (UVT-71): each polygon on the target side gets its 3D mirror partner's UVs, mirrored across the UV symmetry line, one undo step.
  - `axis` (string): The 3D mirror axis: x (default), y or z.
  - `positiveToNegative` (boolean): Copy from the + side to the - side (default true).
  - `mirror` (string): The UV axis mirrored: u (default) or v.
- **`operandi.uv.selectTile`** — Select UVs in the Region (UVT-113): select the polygons whose UVs lie wholly inside the UDIM tile.
  - `udim` (integer): The tile (default 1001).
- **`operandi.uv.nudgeTile`** — Move the selected islands whole tiles over (UVT-113 arrows), one undo step.
  - `u` (integer): Tiles right (negative left).
  - `v` (integer): Tiles up (negative down).
- **`operandi.uv.export`** — Export UVs to SVG or EPS (UVT-120): the current map's edges, the 0-1 tile size units square, V up.
  - `filename` (string): Where to write it.
  - `format` (string): svg (default) or eps.
  - `size` (number): The drawing's width and height (default 1024).
- **`operandi.uv.toMesh`** — Convert UVs to Mesh (UVT-120): a new mesh with a point per UV position (U to X, V to Y) and a polygon per mapped polygon, one undo step. No arguments.
- **`operandi.uv.transfer`** — Transfer UVs (UVT-92): the primary active mesh's current map onto every other active mesh with the same polygons, one undo step. No arguments.
- **`operandi.uv.mirror`** — Mirror the selected UVs across the line axis = center, one undo step.
  - `axis` (string): u (default) or v.
  - `center` (number): The line's position (default 0.5, the last used).
- **`operandi.uv.align`** — Align UVs: set the selected UVs' U (left, right) or V (up, down) to one value of their bounding box, one undo step.
  - `direction` (string): up, down, left or right.
  - `alignTo` (string): low, average, center or high (default the direction's far side).
  - `perRow` (boolean): Each run of connected selected edges on its own.
- **`mesh.retopo`** — Automatic Retopology: rebuilds the active mesh as even quads that follow its shape (QuadriFlow); unnamed attributes keep the last values.
  - `countMode` (string): number or percent (polyTargetNum or polyTargetPct).
  - `polyTargetNum` (integer): About how many quads (default 5000).
  - `polyTargetPct` (number): About this share of the polygon count, 0-10 (default 0.25).
  - `creaseAngle` (number): Radians; new edges follow sharper creases, 0 is off (default 0).
  - `boundaryAlign` (boolean): New edges follow open borders (default true).
  - `vertNormals` (boolean): Smooth shade the result (default true).
  - `newMesh` (boolean): Put the result in a new mesh item (default false).
  - `seed` (integer): Operandi only: random start; the same seed gives the same quads.
- **`sculpt.remesh`** — Voxel Remesh: rebuilds the active mesh as an even quad skin on a voxel grid; joins overlaps and closes holes. Unnamed attributes keep the last values.
  - `density` (number): 0-1, higher is finer (default 0.5).
  - `voxelSize` (number): Operandi only: voxel size in metres; above 0 it replaces density.
  - `keepMaps` (boolean): Operandi only: weight and colour maps carry over (default true).
  - `vertNormals` (boolean): Operandi only: smooth shade the result (default true).
- **`operandi.bake.curvature`** — Bake the active mesh's Curvature map into its UV layout and write it as a 16-bit greyscale PNG: convex edges light, cavities dark, flat 50 % grey. The scene doesn't change.
  - `filename` (string, required): The .png to write.
  - `size` (integer): Width and height in pixels, 16-8192 (default 1024).
  - `padding` (integer): Pixels of padding around each UV island (default 16).
  - `fill` (boolean): Fill the rest of the image smoothly (default true).
  - `uvMap` (string): UV map baked into (default the mesh's Texture map).
  - `rays` (integer): Rays per pixel, 1-1024 (default 32).
  - `radius` (number): Sampling radius as a share of the mesh's size (default 0.001).
- **`operandi.bake.thickness`** — Bake the active mesh's Thickness map into its UV layout and write it as a 16-bit greyscale PNG: thin parts dark, thick parts light. The scene doesn't change.
  - `filename` (string, required): The .png to write.
  - `size` (integer): Width and height in pixels, 16-8192 (default 1024).
  - `padding` (integer): Pixels of padding around each UV island (default 16).
  - `fill` (boolean): Fill the rest of the image smoothly (default true).
  - `uvMap` (string): UV map baked into (default the mesh's Texture map).
  - `rays` (integer): Rays per pixel, 1-1024 (default 64).
  - `distance` (number): Longest distance measured, as a share of the mesh's size (default 0.1).
- **`operandi.bake.padImage`** — Pad a PNG texture around the active mesh's UV islands (seam padding), writing output or the file itself. The scene doesn't change.
  - `filename` (string, required): The .png to pad.
  - `output` (string): Where to write the result (default: replace filename).
  - `padding` (integer): Pixels of padding (default 16).
  - `uvMap` (string): UV map whose islands are used (default the mesh's Texture map).
- **`operandi.remesh`** — Remesh Blocks, Smooth or Sharp: rebuilds the active mesh on an octree-sized grid. Unnamed attributes keep the last values.
  - `mode` (string): blocks, smooth or sharp (default smooth).
  - `depth` (integer): Octree Depth 2-12 (default 6).
  - `scale` (number): How much of the octree the mesh fills, 0-1 (default 0.9).
  - `sharpness` (number): Sharp mode: higher keeps more edges sharp (default 1).
  - `removeDisconnected` (boolean): Drop small loose pieces (default false).
  - `threshold` (number): With removeDisconnected: smallest piece kept, as a share of the biggest, 0-1 (default 1).
  - `vertNormals` (boolean): Smooth shade the result (default true).
- **`poly.boolean`** — Boolean of the active mesh with its drivers (BOOL-25); the drivers are untouched.
  - `mode` (integer|string): 0 union (default), 1 intersect, 2 subtract, 3 add.
  - `cutmesh` (integer|string): Drivers: 0 background (default: visible unselected meshes), 1 lastSelected (the selected pieces of the active mesh), 2 item.
  - `item` (integer): Driver item id for cutmesh item (a group drives with its meshes).
  - `children` (boolean): Add the drivers' child meshes; default false.
- **`poly.solidDrill`** — Solid Drill: cut the active mesh's polygons with a closed driver (DRILL-06); no driver faces are added.
  - `mode` (integer|string, required): 0 core, 1 tunnel, 2 stencil, 3 slice (or the word).
  - `material` (string): Stencil's material for the inside pieces; default Stencil, created if missing.
  - `cutmesh` (integer|string): Drivers: 0 background (default), 1 lastSelected, 2 item.
  - `item` (integer): Driver item id for cutmesh item.
  - `children` (boolean): Add the drivers' child meshes; default false.
- **`poly.drill`** — Axis Drill: cut the active mesh's polygons with the driver outline's prism along an axis (DRILL-06).
  - `mode` (integer|string, required): 0 core, 1 tunnel, 2 stencil, 3 slice (or the word).
  - `axis` (integer|string, required): 0 x, 1 y, 2 z (or the letter).
  - `material` (string): Stencil's material for the inside pieces; default Stencil, created if missing.
  - `cutmesh` (integer|string): Drivers: 0 background (default), 1 lastSelected, 2 item.
  - `item` (integer): Driver item id for cutmesh item.
  - `children` (boolean): Add the drivers' child meshes; default false.
- **`poly.setMaterial`** — Set the material tag of the selected polygons (a new material when the name is new).
  - `name` (string, required): Material name.
- **`poly.unify`** — Remove polygons that duplicate another (same points, either winding), keeping the lowest of each group; one undo step.
  - `force` (boolean): Also remove duplicates whose tags or corner data differ (default false).
- **`mesh.cleanup`** — Mesh Cleanup: run the checked operations on the selected polygons (the whole active mesh when none), one undo step (BRQ-60). Each switch not given takes the dialog's Reset value, on; all off is refused.
  - `floatingVertex` (boolean): Remove Floating Vertices (default true).
  - `onePointPolygon` (boolean): Remove 1 Point Polygons (default true).
  - `twoPointPolygon` (boolean): Remove 2 Point Polygons (default true).
  - `dupPointPolygon` (boolean): Fix Duplicate Points in Polygons (default true).
  - `colinear` (boolean): Remove Colinear Vertices (default true).
  - `faceNormal` (boolean): Make Face Normals Consistent (default true).
  - `mergeVertex` (boolean): Merge Overlapping Vertices (default true).
  - `mergeDisco` (boolean): Merge Discontinuous UVs (default true).
  - `unifyPolygon` (boolean): Unify Polygons (default true).
  - `forceUnify` (boolean): Force Unify (default true).
  - `removeDiscoWeight` (boolean): Remove Discontinuous Weights (default true).
  - `fixGaps` (boolean): Fix Gaps (default true).
- **`chamfer.edit`** — Chamfer Edit once on the selected polygons; one undo step. Options not given take the Reset values.
  - `offset` (number): Metres (default 0).
  - `segments` (integer): Default 1.
  - `sourceSegs` (integer): Default 1.
  - `depth` (number): Factor, 1 = 100 % (default 1).
  - `mode` (string): inset (default) or width: what Offset changes.
  - `absSize` (boolean): Absolute Size: Offset is the inset or width itself (default false).
  - `swapSide` (boolean): Swap Side: the strip runs the other way; Offset moves its open ends (default false).
  - `reverseX` (boolean): Reverse Direction; no effect without a profile (default false).
  - `reverseY` (boolean): Reverse Depth; no effect without a profile (default false).
- **`poly.haul`** — Poly Haul once on the selected polygons; one undo step.
  - `offset` (number, required): Metres.
- **`@ppi_flatten_ngons.py`** — Flatten the selected polygons of more than four sides. No arguments.
- **`operandi.mesh.fixBlackDots`** — Fix Black Dots (MT2): every pair of polygons sharing two or more edges in a row (inner points used by those two only) becomes one polygon; the inner points are kept unused and selected with the run's ends, Vertex mode, one undo step. 'No black dots found.' when there are none. No arguments.
- **`operandi.edge.zip`** — Edge Zip (MT3): each selected edge needs a quad on each side; its points and the points across those quads weld at their average, the quads collapse, one undo step, nothing selected afterwards. No arguments.
- **`operandi.poly.diagonalRedirect`** — Diagonal Re-Direct (MT4): two selected quads touching at one corner split along the diagonal through it; each side quad off the border gets a centre point and is rebuilt so the diagonal is an edge loop, one undo step. No arguments.
- **`poly.split`** — Split the selected polygons along selected vertices. No arguments.
- **`poly.quadruple`** — Turn selected triangle pairs into quads. No arguments.
- **`poly.spinQuads`** — Spin the edge shared by two selected quads. No arguments.
- **`poly.align`** — Make the selected polygons face the same way as the first. No arguments.
- **`poly.triple`** — Triangulate the selected polygons. No arguments.
- **`poly.flip`** — Flip the selected polygons' facing. No arguments.
- **`poly.remove`** — Remove the selected polygons. No arguments.
- **`poly.merge`** — Merge the selected polygons into one. No arguments.
- **`poly.collapse`** — Collapse each selected polygon to a point. No arguments.
- **`poly.make`** — Make a polygon from the selected vertices, in selection order. No arguments.
- **`poly.makeCurveOpen`** — Make an open Curve through the selected vertices in selection order; Polygon mode with the curve selected after. No arguments.
- **`poly.makeCurveClosed`** — Make a closed Curve through the selected vertices in selection order; Polygon mode with the curve selected after. No arguments.
- **`poly.makeCurveFill`** — Add a Curve Fill polygon to each selected closed curve. No arguments.
- **`curve.winding`** — Align the selected curves' winding: counter-clockwise seen from the automatic axis (clockwise with reverse).
  - `merge` (boolean): Join open curves that share end points first (default true).
  - `reverse` (boolean): Wind the other way (default false).
- **`operandi.uv.rectangle`** — UV Rectangle on the selected polygons (all when none): each island that is a full grid of quads becomes a grid of rectangles.
  - `name` (string): UV map (default Texture).
  - `pack` (boolean): Pack the islands into 0..1 after (default false).
  - `gaps` (number): Pack gap, 0-1 (default 0.2).
  - `uniformX` (boolean): Equal spans along U (default false).
  - `uniformY` (boolean): Equal spans along V (default false).
  - `orient` (boolean): Line each island up with U (default false).
- **`operandi.dicer`** — Dicer: cut the active (closed) mesh with random planes into closed chunks; the same seed gives the same chunks.
  - `seed` (integer): Random Seed (default 40).
  - `planes` (integer): Max. Cutting Planes, 1-16 (default 6).
  - `gap` (number): Space between chunks (default 0.001).
  - `innerMaterial` (string): Material of the new inside faces.
  - `scaleX` (number): Fragment Scale X (default 1).
  - `scaleY` (number): Fragment Scale Y (default 1).
  - `scaleZ` (number): Fragment Scale Z (default 1).
- **`operandi.vertMap.transfer`** — Transfer Vertex Map from the first shown background mesh to the active mesh's selected points (all when none), sampling the source surface (values interpolate across its polygons).
  - `source` (string): The map's name (Position needs none).
  - `type` (string): position, weight, uv, morph, rgb or rgba (default position).
  - `space` (string): world (the items' placements count, default) or local (GO-33).
  - `method` (string): distance (nearest spot, default) or raycast (along each point's normal; misses get no value, GO-34).
- **`vert.set`** — Set Position: give every selected point (or the points of the selected edges or polygons) one coordinate; one undo step.
  - `axis` (string, required): x, y or z.
  - `value` (number, required): Metres.
  - `workPlane` (boolean): Along the work plane's axis from its centre (default false).
  - `worldSpace` (boolean): A world coordinate (default false).
- **`poly.subdivide`** — Subdivide the selected polygons (all when none), one undo step (BRQ-10). Options not given take the Subdivide dialog's Reset values (Catmull-Clark, fractal 0, max angle 89.5°, Smooth All), not the last used.
  - `type` (string): Method: "ccsds" (Catmull-Clark, the default; the SDS Subdivide button), "flat" (Faceted), "smooth" or "sds" (the dialog's SDS Subdivide).
  - `fractal` (number): Fractal, 0-1 (default 0).
  - `maxangle` (number): Max Smoothing Angle, radians (default 89.5°).
  - `linear` (boolean): Linear UVs (default false).
  - `bound` (string): Boundary rule (Catmull-Clark): "smooth" (Smooth All, default), "crease" (Crease All) or "creaseEdges".
  - `noEdgeWeight` (boolean): Ignore edge weights (default false).
  - `iterations` (integer): How many times to subdivide, 1-8 (default 1; an Operandi extra: the button's repeat count).
- **`poly.unsubdiv`** — UnSubdivide: merge groups of four quads back into one, one undo step (BRQ-10). Options not given take the dialog's Reset values (iteration 1, uv and material hints on).
  - `iteration` (integer): Iterations, 0-16 (default 1).
  - `flat` (boolean): Flat (default false).
  - `uv` (boolean): UV hint (default true).
  - `material` (boolean): Material hint (default true).
  - `part` (boolean): Part hint (default false).
  - `smoothingGroup` (boolean): Smoothing group hint (default false).
  - `selectionSet` (boolean): Selection set hint (default false).
  - `selectionSetName` (string): The selection set the hint reads.
- **`scale.absolute`** — Absolute Scaling: scale the selection (or the active mesh) to typed sizes, one undo step. mode grab only reads: it answers the selection's size and changes nothing. Values not given take the Absolute Scaling window's defaults (0 sizes leave that axis alone, centre Center).
  - `mode` (string, required): "grab", "uniform", "explicit" or "reference".
  - `absolute_size_uniform` (number): uniform: the length, metres, the Uniform Axis becomes.
  - `absolute_size_uniform_pref` (string): Uniform Axis: "longestAxis" (default), "x", "y" or "z".
  - `orient_to_selection` (boolean): Measure along the selection's own sides; default false.
  - `absolute_size_x` (number): explicit / reference: X size, metres (0 leaves X alone).
  - `absolute_size_y` (number): explicit / reference: Y size, metres (0 leaves Y alone).
  - `absolute_size_z` (number): explicit / reference: Z size, metres (0 leaves Z alone).
  - `absolute_size_center_x` (string): What stays put along X: "center" (default), "high", "low" or "origin" (the item's origin).
  - `absolute_size_center_y` (string): Along Y: "center", "high", "low" or "origin".
  - `absolute_size_center_z` (string): Along Z: "center", "high", "low" or "origin".
- **`vert.center`** — Center: move the selected points, edges or polygons (the whole mesh when none) so their world bounding-box centre is 0 on the given axes, one undo step (BRQ-10).
  - `axis` (string, required): "all", "x", "y", "z", "xy", "yz" or "zx".
- **`poly.freeze`** — Freeze curves and subdivision surfaces of the active mesh into plain polygons (the selected polygons in Polygon mode, else all), one undo step. Arguments not given keep the last Freeze's values (defaults: faces, tess true, fixed true, 5°). Never opens the dialog here.
  - `face` (string): Curves become 0 / "faces", 1 / "polylines" or 2 / "twoPointPolygons" (2 is Operandi's).
  - `tess` (boolean): Freeze subdivision surfaces too (Tessellation); default true.
  - `fixed` (boolean): Fixed spans per curve segment; false uses cangle instead.
  - `cangle` (number): Curve Refinement Angle, radians (above 0, at most π), used when fixed is false.
  - `drill` (boolean): Accepted and ignored (not built yet).
  - `axis` (integer): Accepted and ignored (not built yet).
  - `disp` (boolean): Accepted and ignored (not built yet).
  - `deform` (boolean): Accepted and ignored (not built yet).
  - `morphSave` (boolean): Accepted and ignored (not built yet).
  - `morphName` (string): Accepted and ignored (not built yet).
- **`edge.addLoop`** — Add an edge loop across the edge a–b.
  - `a` (integer, required): Edge start vertex index.
  - `b` (integer, required): Edge end vertex index.
  - `t` (number, required): Position along a→b, 0–1.
- **`edge.addPoint`** — Add a point on the edge a–b.
  - `a` (integer, required): Edge start vertex index.
  - `b` (integer, required): Edge end vertex index.
  - `t` (number, required): Position along a→b, 0–1.
- **`edge.growQuads`** — Grow quads from the selected boundary edges. No arguments.
- **`edge.fillQuads`** — Fill the hole bounded by the selected edges with quads. No arguments.
- **`edge.split`** — Split the mesh along the selected edges. No arguments.
- **`edge.spinQuads`** — Spin the selected edges. No arguments.
- **`edge.join`** — Join each selected edge's ends.
  - `average` (boolean): At their average position (true) or not (false, default).
- **`edge.collapse`** — Collapse the selected edges. No arguments.
- **`edge.remove`** — Remove the selected edges. No arguments.
- **`edge.unbevel`** — Undo a bevel on the selected edges. No arguments.
- **`edge.subdivide`** — Subdivide the selected edges.
  - `count` (integer): New points per edge; omit for the plain one-cut Subdivide.
  - `percent` (number): With count 1: where the point goes along the edge, 0–1 (0.5 = middle).
- **`vert.new`** — Add a vertex to the active mesh.
  - `position` ([x, y, z]): Item-space position, metres (or give x, y and z).
  - `x` (number): Metres, with y and z.
  - `y` (number): Metres.
  - `z` (number): Metres.
- **`vert.split`** — Split the selected vertices. No arguments.
- **`vert.splitEdge`** — Split the edges at the selected vertices. No arguments.
- **`vert.align`** — Align the selected vertices.
  - `axes` ([bool, bool, bool]): Which of X, Y, Z to align; default all.
  - `uniform` (boolean): Uniform spacing; default false.
- **`vert.merge`** — Merge selected vertices closer than a distance.
  - `dist` (number): Metres; default 0 (coincident only).
- **`vert.join`** — Join the selected vertices into one.
  - `average` (boolean): At their average position (true) or the first (false, default).
- **`vert.collapse`** — Collapse the selected vertices. No arguments.
- **`vert.remove`** — Remove the selected vertices (and their polygons). No arguments.
- **`vert.removeEdge`** — Remove the selected vertices, keeping polygons. No arguments.
- **`delete`** — Delete the selection of the current mode: points take their polygons, edges dissolve, polygons go; unused points go too; items are deleted (the Delete and Backspace keys). No arguments.
- **`vert.weld`** — Weld vertices onto a target vertex.
  - `sources` (array of integers, required): Vertex indices to weld.
  - `target` (integer, required): Vertex index they move to.
- **`item.create`** — Add an item from the Items list's Add menu (camera, a light, a mesh...), alone selected.
  - `type` (string, required): Item type key; see the `types` list in command_help item.create.
- **`item.pattern`** — Select, unselect or hide the items whose names match a pattern (Items list header).
  - `action` (string, required): "select", "unselect" or "hide".
  - `pattern` (string): Name pattern with * and ? wildcards; default "*".
  - `limit_to_children` (boolean): Only descendants of the selected items; default false.
- **`mesh.new`** — Add an empty mesh item and make it active.
  - `name` (string): Name for the new mesh; default "Mesh" (numbered when taken).
- **`select.item`** — Make an item the active (selected) item (the map's Select Item; was item.select, still accepted). Doesn't change the component mode; the select tool's item argument does the same in any mode.
  - `item` (integer): Item id; omit to clear the item selection.
  - `mode` (integer): Accepted, not used yet (the map's mode).
  - `mask` (string): Accepted, not used yet (the map's mask).
- **`item.delete`** — Delete the active item; with `children`, every selected item.
  - `children` (string): "delete" deletes their child items too; "root" moves them to the scene root in place (default).
  - `item` (string): A mesh operation to delete instead, written op:<item>:<op> (Mesh Stack).
- **`item.name`** — Rename the active item, or a shader-tree layer with layer.
  - `name` (string, required): New name, 1–1024 bytes.
  - `layer` (integer): A shader-tree layer id to rename instead (Operandi's own argument).
- **`item.visibility`** — Toggle an item's visibility.
  - `item` (integer): Item id; default the active item.
- **`item.parent`** — Parent an item to another (or to the scene root).
  - `item` (integer): Item id; default the active item.
  - `parent` (integer): New parent's id; omit for the scene root.
  - `keep_position` (boolean): Keep the item where it is in the world; default true.
  - `position` (integer): With item:op:<item>:<op>, the op's new place in evaluation order (0 runs first).
- **`meshop.create`** — Add a mesh operation to the selected mesh, directly above the selected row of its Mesh Operations list (on top when none is selected). Polygons selected on the Base Mesh become its Select by Index.
  - `type` (string, required): Op type, such as prim.cube.item, poly.bevel.item, pmodel.meshmerge (Merge Meshes), boolean.meshop.item, solidDrill.meshop.item or axisDrill.meshop.item; command_help meshop.create lists every type (types).
  - `mode` (integer): Accepted, not used yet.
  - `category` (string): Accepted, not used yet.
  - `applyDefaultPreset` (boolean): Accepted, not used yet.
- **`deformer.enable`** — Turn a mesh operation on or off (its eye); off passes its input through.
  - `deformer` (string): The op, op:<item>:<op>; default the selected one.
  - `enable` (boolean): Default true.
- **`deformer.freeze`** — Freeze a mesh operation and every op below it into the Base Mesh; the ops above stay live. With duplicate, the frozen copy is a new mesh item.
  - `duplicate` (boolean): Freeze into a new mesh item; default false.
  - `deformer` (string): The op, op:<item>:<op>; default the selected one.
  - `mesh` (integer): Accepted, not used yet.
- **`mesh.freeze`** — Freeze the selected mesh's whole Mesh Stack into a plain mesh. No arguments.
- **`mirror.clone.create`** — Add a Mirror mesh operation (a Clone Effector with a pmodel.mirror generator) to the selected mesh, as meshop.create does. No arguments.
- **`array.clone.create`** — Add an Array mesh operation (a Clone Effector with a pmodel.array generator) to the selected mesh, as meshop.create does. No arguments.
- **`deformer.selectBaseMesh`** — Edit Base Mesh on or off: direct tools then edit the mesh's Base Mesh, with the stack's result drawn as a ghost.
  - `item` (integer): Item id; default the active item.
  - `select` (boolean): Default true.
- **`selop.indexcreate`** — Give the selected mesh operation a Select by Index of the polygons selected on its Base Mesh. No arguments.
- **`selop.selection`** — Edit a mesh operation's Select by Index from the polygons selected on its Base Mesh.
  - `mode` (string): set (default), clear, add or remove; or 0 to 3.
  - `item` (string): The op, op:<item>:<op>; default the selected one.
- **`selop.create`** — Add a selection operation (index.selop.item, selbyrange.selop.item, pattern.selop.item, previous.selop, operandi.selset.selop) on top of a mesh operation's Selection input.
  - `type` (string, required): The selection operation's type.
  - `item` (string): The op, op:<item>:<op>; default the selected one.
- **`selop.edit`** — Change values of, or remove, a mesh operation's selection operation (0 = bottom).
  - `index` (integer, required): Which selection operation, from the bottom (0).
  - `values` (object): Values to set: enable, invert, blendmode (override, add, subtract, multiply), and the kind's own (index, start, end, step, pattern, source, name).
  - `remove` (boolean): Remove it instead.
  - `item` (string): The op, op:<item>:<op>; default the selected one.
- **`item.drop`** — Run a Drop Action, as when dragging items onto another in the Items list.
  - `action` (string, required): Drop action key; see the `actions` list in command_help item.drop.
  - `destination` (integer, required): The item dropped onto.
  - `sources` (array of integers): Items dropped; default the item selection in order.
  - `ray_origin` ([x, y, z]): Place / Place and Align only: world-space ray start, metres.
  - `ray_direction` ([x, y, z]): Place / Place and Align only: world-space ray direction.
- **`item.transform`** — Set an item's transform (one undo step). Values are absolute; channels not given keep their value.
  - `item` (integer): Item id; default the active item.
  - `translation` ([x, y, z]): Position relative to the parent, metres, absolute.
  - `rotation` ([x, y, z]): Euler angles, radians, absolute (X then Y then Z, as scene_info reports).
  - `scale` ([x, y, z]): Scale factors, absolute (1 = unscaled).
- **`symmetry.set`** — Set modelling symmetry.
  - `axis` (string): "off" (default), "x", "y" or "z".
  - `offset` (number): Symmetry plane offset along the axis, metres; default 0.
- **`select.symmetrical`** — Add the symmetrical counterparts of the selection. No arguments.
- **`operandi.anim.query`** — Read animation state as JSON: current frame, fps, scene and current range, time display, Auto Key, snap, real time, end behavior, key time, playing, pending edits, markers, and each animated channel's keys (frame, value, slopes), value now and values at frames.
  - `channels` (string): Only these channels, item:name,...
  - `item` (integer): Only this item's channels.
  - `frames` (string): Frames to read each curve at, comma separated.
- **`operandi.render.frameRange`** — Render frames start..end (default the current range) with Cycles into folder (default the project's renders/) as <name>_0000.png; needs the running app. operandi.anim.query reports progress (job).
  - `start` (number): First frame (default the current range's).
  - `end` (number): Last frame (default the current range's).
  - `range` (string): current (default) or scene: which range start and end default to.
  - `folder` (string): Folder to write to.
  - `name` (string): File name before the frame number (default Render).
  - `samples` (integer): Samples per pixel (default the render settings').
  - `width` (integer): Width in pixels (with height).
  - `height` (integer): Height in pixels (with width).
  - `camera` (integer): Camera item (default the render camera).
- **`operandi.render.cancelRange`** — Stop the running frame range render or playblast. No arguments.
- **`operandi.view.playblast`** — Record the active 3D view for frames start..end (default the current range) at scale 1, 0.5 or 0.25 to an image sequence in folder (default the project's renders/), then open it unless open is false; needs the running app.
  - `start` (number): First frame.
  - `end` (number): Last frame.
  - `range` (string): current (default) or scene.
  - `scale` (number): 1, 0.5 or 0.25 of the view's size (default 1).
  - `folder` (string): Folder to write to.
  - `name` (string): File name before the frame number (default Playblast).
  - `open` (boolean): Open the result in the system player after (default true).
  - `format` (string): movie (default: an .mp4, through ffmpeg when it is installed; else the images) or images (a PNG sequence).
- **`operandi.key.offset`** — Move the selected keys by frames in time and by value, one undo step (GE-40/45).
  - `frames` (number): Frames to move by (default 0).
  - `value` (number): Value to add (default 0).
- **`operandi.key.setValue`** — Sets, adds to or multiplies the selected keys' values (GE-71 Set Value).
  - `value` (number, required): The value.
  - `mode` (string): set (default), add or multiply.
- **`operandi.key.scaleValue`** — Scales the selected keys' values around a value (GE-43).
  - `factor` (number, required): Factor.
  - `pivot` (number): The value to scale around (default 0).
- **`operandi.key.setFrame`** — Puts the selected keys at a frame (the earliest there, spacing kept), or moves them by frames (GE-71 Set Frame).
  - `frame` (number, required): Frame (or frames to move by with relative).
  - `relative` (boolean): Move by frame instead of to it.
- **`operandi.key.handle`** — Sets the selected keys' in or out handle: slope (value per frame) and/or weight; that side becomes Manual, a unified key's other side follows the slope (GE-50/53).
  - `side` (string): in or out (default out).
  - `slope` (number): Slope, value per frame.
  - `weight` (number): Weight 0.01-3 (1 is the plain length).
- **`operandi.key.break`** — Breaks the selected keys' handles so each side moves alone (GE-51). No arguments.
- **`operandi.key.unify`** — Unifies the selected keys' handles into one line (GE-51). No arguments.
- **`operandi.key.autoHandles`** — Puts the selected keys' handle weights back to Auto; with slopes:true their slopes too (GE-51/52).
  - `slopes` (boolean): Slopes back to Auto as well.
- **`operandi.key.add`** — Adds a key on a channel at a time and value (GE-31); the curve's own value when value is left out.
  - `channel` (string, required): item:name.
  - `frame` (number): Time in frames.
  - `time` (number): Time in seconds (instead of frame).
  - `value` (number): The key's value.
- **`operandi.key.keepShape`** — Keys channels at a time without changing their curves (GE-32 Key Keeping Shape).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
  - `frame` (number): Time in frames.
  - `time` (number): Time in seconds (instead of frame).
- **`operandi.curve.behavior`** — Sets curves' behaviour before the first key and/or after the last (GE-61).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
  - `pre` (string): constant, constantSmooth, linear, repeat, oscillate, oscillateSmooth, offsetRepeat, offsetRepeatSmooth or reset.
  - `post` (string): As pre.
- **`operandi.curve.interpolation`** — Sets curves' interpolation: every key's slopes (GE-60).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
  - `type` (string, required): curve, linear or stepped.
- **`operandi.curve.cycle`** — Turns the cycle option on or off: the last key matches the first (GE-62).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
  - `on` (boolean): On (default true).
- **`operandi.curve.shadow`** — Copy to Shadow, Replace with Shadow, Swap or Clear Shadow (GE-80).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
  - `action` (string): copy (default), replace, swap or clear.
- **`operandi.curve.eulerFilter`** — Unwraps rotation curves that jump by whole turns (GE-X1).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
- **`operandi.curve.simplify`** — Removes keys that change the curve by no more than a tolerance (GE-X2).
  - `channels` (string): Channels as item:name,...; default the selected channels, else the selected keys' channels.
  - `tolerance` (number): Largest change allowed (default 0.001).
- **`operandi.time.set`** — Sets the current time (TK-01); not an undo step (TK-05). Snaps to a frame with Time Snap on.
  - `frame` (number): Time in frames.
  - `time` (number): Time in seconds (instead of frame).
- **`operandi.time.step`** — Steps the current time by frames (TK-63; Shift+Left/Right, comma/period).
  - `frames` (number): Frames to step (default 1; negative steps back).
- **`operandi.time.go`** — Goes to the current range's first or last frame, or the previous, next, first or last key of the selected items or channels (TK-63).
  - `to` (string): first, last, prevKey, nextKey, firstKey or lastKey (default first).
- **`operandi.time.play`** — Plays or stops playback through the current range (TK-60; /).
  - `backwards` (boolean): Play backwards (default false).
  - `stop` (boolean): true stops, false starts; default toggles.
- **`operandi.time.keyTime`** — Sets the key time alone, where new keys land (TL-20), or resets it to follow the current time.
  - `frame` (number): Time in frames.
  - `time` (number): Time in seconds (instead of frame).
  - `reset` (boolean): Follow the current time again.
- **`operandi.channel.select`** — Selects a channel for keying (UIC-119), or clears the channel selection.
  - `channel` (string): item:name, or none.
  - `add` (boolean): Toggle it in the selection (default false).
- **`operandi.key.select`** — Selects keys in the track row (TL-41).
  - `keys` (string): Keys as item:name@frame,....
  - `mode` (string): replace, add, remove, all or clear (default replace).
- **`operandi.marker.select`** — Selects markers (TL-52).
  - `ids` (string): Marker ids, comma separated.
  - `add` (boolean): Add to the selection (default false).
- **`operandi.key.copy`** — Copies the selected keys (or the selected items' keys at the key time) to the key clipboard (TL-45/47); not an undo step. No arguments.
- **`operandi.key.set`** — Sets keys at the key time with the channels' current values (TK-20/21; S keys the selected channels).
  - `channels` (string): Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels.
- **`operandi.key.toggle`** — A status-light click: keys the channels, or removes their keys at the key time when every one has one (TK-20).
  - `channels` (string): Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels.
- **`operandi.key.remove`** — Removes the channels' keys at the key time or the given time (TK-24 Remove Key).
  - `channels` (string): Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels.
  - `frame` (number): Time in frames.
  - `time` (number): Time in seconds (instead of frame).
- **`operandi.key.transforms`** — Keys position, rotation, scale or all three of the selected items (TK-22; Shift+W/E/R/Y).
  - `type` (string): position, rotation, scale or all (default all).
- **`operandi.key.items`** — Keys every animated channel of the selected items (TK-23; Shift+S). No arguments.
- **`operandi.key.removeAnimation`** — Removes every key of the channels, keeping their current values (TK-24).
  - `channels` (string): Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels.
- **`operandi.key.reset`** — Removes every key of the channels and puts back their default values (TK-24).
  - `channels` (string): Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels.
- **`operandi.key.bake`** — Keys the channels on every frame of the current range (TK-24).
  - `channels` (string): Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels.
- **`operandi.key.apply`** — Keys every edited-but-not-keyed channel at the key time (TK-41 Apply). No arguments.
- **`operandi.key.discard`** — Drops every edited-but-not-keyed value back to its curve (TK-41 Discard). No arguments.
- **`operandi.key.autoKey`** — Sets Auto Key: off, animated or all (TK-30).
  - `mode` (string, required): off, animated or all.
- **`operandi.key.move`** — Moves the selected keys by frames (TL-42, Shift...).
  - `frames` (number, required): Frames to move (negative moves earlier).
- **`operandi.key.scale`** — Scales the selected keys' timing around a pivot (TL-43, Scale...).
  - `factor` (number, required): Timing factor, above 0.
  - `frame` (number): Pivot frame (default the key time).
  - `time` (number): Time in seconds (instead of frame).
- **`operandi.key.invert`** — Reverses the selected keys' timing within their span (TL-45 Invert...). No arguments.
- **`operandi.key.delete`** — Deletes the selected keys (TL-45 Delete / Delete Related).
  - `related` (boolean): Also the sibling X/Y/Z keys at the same times.
- **`operandi.key.slope`** — Sets the selected keys' slope type (TL-45/46).
  - `type` (string, required): auto, smoothFlat, autoFlat, flat, linear, linearIn, linearOut, stepped or manual.
  - `side` (string): in, out or both (default both; linearIn/Out pick theirs).
- **`operandi.key.cut`** — Copies the selected keys to the key clipboard, then deletes them (TL-45). No arguments.
- **`operandi.key.paste`** — Pastes the key clipboard at the key time (TL-45): merge, insert (later keys move along) or replace (keys in the pasted span go).
  - `mode` (string): merge, insert or replace (default merge).
- **`operandi.time.insertFrame`** — Moves the selected items' keys after the key time one frame later (TL-21). No arguments.
- **`operandi.time.removeFrame`** — Moves the selected items' keys after the key time one frame earlier, never deleting keys (TL-21). No arguments.
- **`operandi.time.settings`** — Sets the scene's time settings (TK-03/04): frame rate, scene and current range (frames), time display, Auto Key, Time Snap, Play Real Time, End Behavior, Auto Update Existing Keys.
  - `fps` (number): Frames per second.
  - `start` (number): Scene range start, frames.
  - `end` (number): Scene range end, frames.
  - `rangeStart` (number): Current range start, frames.
  - `rangeEnd` (number): Current range end, frames.
  - `display` (string): frames, seconds, smpte or film.
  - `autoKey` (string): off, animated or all.
  - `snap` (boolean): Time Snap.
  - `realTime` (boolean): Play Real Time.
  - `endBehavior` (string): hold, repeat or wrap.
  - `autoUpdate` (boolean): Auto Update Existing Keys.
- **`operandi.time.range`** — Sets the current range in frames (TL-13), or resets it to the scene range.
  - `start` (number): Start frame.
  - `end` (number): End frame.
  - `reset` (boolean): Back to the scene range.
- **`operandi.time.fit`** — Fits the current range to the selected items' or channels' keys, or every key (TL-14; A / Shift+A).
  - `all` (boolean): Every key in the scene.
- **`operandi.marker.add`** — Adds a time marker at the current or given time, owned by the selected item or the scene (TL-50/51; M).
  - `frame` (number): Time in frames.
  - `time` (number): Time in seconds (instead of frame).
  - `scene` (boolean): Owned by the scene even with items selected.
- **`operandi.marker.edit`** — Changes markers (TL-52/53): time, shift, length, end, label, shape, colour, Show Label, Frame Limits, Snap to Frames.
  - `ids` (string): Marker ids; default the selected markers.
  - `frame` (number): New start frame.
  - `shift` (number): Frames to move by.
  - `length` (number): Range length, frames (0: a plain marker).
  - `endFrame` (number): Range end frame.
  - `label` (string): Label.
  - `shape` (string): triangle, square, diamond or bar.
  - `color` (string): r,g,b 0-1.
  - `showLabel` (boolean): Show the label above the marker.
  - `frameLimits` (boolean): Limit playback and stepping to the range.
  - `snapToFrames` (boolean): Move start and end onto whole frames.
- **`operandi.marker.delete`** — Deletes markers (TL-52).
  - `ids` (string): Marker ids; default the selected markers.
- **`transform.channel`** — Set one transform channel of an item (default the active item); value "?" answers the current value. One undo step; on the Texture Locator a live Move/Rotate/Scale/Transform holds, one step inside that tool, as a drag of its handle (PD-Q12).
  - `name` (string, required): pos.X/Y/Z (metres), rot.X/Y/Z (radians, or with deg) or scl.X/Y/Z (a Texture Locator's Size, at least 0.001).
  - `value` (any, required): The new value; "?" reads it.
  - `mode` (integer): Not supported yet (the map's mode); leave it out.
  - `item` (integer): Item id; default the active item.
  - `filter` (string): Not supported yet (the map's filter); leave it out.
  - `replace` (string): Not supported yet (the map's replace); leave it out.
- **`transform.reset`** — Reset part of every selected item's transform to identity (position 0, rotation 0, scale 100%). Children follow; geometry is untouched; locked items are skipped. One undo step, none when nothing changes.
  - `type` (string): translation, rotation, scale or all; default all.
- **`transform.freeze`** — Bake part of every selected mesh item's transform into its points, then set that part to identity, so nothing moves on screen. Children are compensated; mirrored scale reverses polygons; locked items and non-mesh items are skipped. One undo step, none when nothing changes.
  - `type` (string): translation, rotation, scale or all; default all.
- **`item.setType`** — Change a light's type, keeping what both types share (Light Properties > Type).
  - `type` (string, required): Light type: Directional, Point, Spot, Area, Cylinder, Photometric, Mesh, Dome or Portal.
  - `mask` (string): Not supported yet (the map's mask); leave it out.
  - `item` (integer): Item id; default the active item.
- **`light.temperature`** — Set a light's colour from a colour temperature (Light Material > Set Temperature).
  - `kelvins` (number, required): Colour temperature, kelvins.
  - `gamma` (number): Not supported yet (the map's gamma); leave it out.
  - `item` (integer): Item id; default the active item.
- **`light.mesh`** — Set a Mesh Light's Prototype mesh (or none).
  - `light` (integer): The Mesh Light's item id; default the active item.
  - `mesh` (string, required): The mesh item id, or "none".
- **`texture.setIMap`** — Set an Image Map layer's Image: a scene image by name or file path, or none.
  - `image` (string, required): Scene image name or file path, or "none".
  - `name` (string): Not supported yet (the map's name); leave it out.
  - `layer` (integer, required): The Image Map layer's id.
- **`texture.delete`** — Delete a texture layer (Shader Tree or environment) and its unshared Texture Locator.
  - `item` (integer, required): The texture layer's id.
- **`texture.setLocator`** — Set a texture layer's Texture Locator item (or none).
  - `locator` (string, required): Locator item id, or "none".
  - `layer` (integer, required): The texture layer's id.
- **`texture.setVMap`** — Set a Vertex Map texture layer's map by name (or none).
  - `name` (string, required): Vertex map name, or "none".
  - `layer` (integer, required): The Vertex Map layer's id.
- **`mask.setPTag`** — Set a Group's Polygon Tag: a tag name, "(all)" or "(none)".
  - `name` (string, required): Tag name, (all) or (none).
  - `layer` (integer, required): The Group layer's id.
- **`mask.setPTagType`** — Set a Group's Polygon Tag Type (Material, Part or Selection Set); the tag goes back to (none).
  - `name` (string, required): Tag type.
  - `layer` (integer, required): The Group layer's id.
- **`item.channel`** — Set one channel of an item (default the active item) or of a shader layer; value "?" answers the current value. Channel names are Operandi's own for now (docs/COMMAND_REGISTRY.md lists them).
  - `name` (string, required): Channel name, e.g. intensity, color (color.r for one part), opacity.
  - `value` (any, required): The value: number, true/false, text, r,g,b, an item id or none; "?" reads it. Percents 0–1, distances metres, angles radians.
  - `mode` (integer): Not supported yet (the map's mode); leave it out.
  - `item` (integer): Item id; default the active item.
  - `layer` (integer): A shader-tree layer id instead of an item (Operandi's own argument).
  - `scene` (boolean): true: the scene's own time channels fps, start, end, time (seconds) instead of an item (Operandi's own argument).
- **`environment.light`** — Set Physical Sun: the directional light whose direction places the sun of a Physically-based Daylight environment (environments-2026-11-09 §3). One undo step.
  - `envMaterial` (integer): The Environment Material's layer id; default the active environment's.
  - `sunLight` (any, required): A directional light's item id, or none.
- **`smoothing.set`** — Set the smooth-shading rule (one for every mesh: the default material's smoothing). Fields not given keep their value. Viewport, exports and renders all use it.
  - `amount` (number): How far corners bend toward the averaged normal: 0 flat, 1 fully smooth (the default); -1..2 allowed.
  - `angle` (number): Smoothing angle, radians: neighbouring polygons within this angle blend (default 40° = 0.698).
  - `crease_by_angle` (boolean): Off blends every connected polygon (creases only from groups); default true.
  - `groups` (string): Smoothing groups: "off", "crease" (default) or "force".
- **`operandi.nodeMaterial.import`** — Import a node graph file (JSON) as Node Material layers, placed as Add Layer places one; all or nothing per material. One undo step.
  - `filename` (string, required): The node graph file.
  - `layer` (integer): A shader-tree layer the new layers go next to; default as Add Layer with nothing selected.
- **`operandi.nodeMaterial.setDefault`** — Set a node graph's shared exposed default (layers without their own value follow), one undo step; value "?" answers the current default.
  - `graph` (integer, required): The graph's id (shader_tree_get lists node graphs).
  - `input` (string, required): The exposed input's name, e.g. Tint.
  - `value` (any, required): A number, r,g,b text or [r, g, b] (0..1 components); "?" reads it.
- **`operandi.nodes.add`** — Add a free MaterialX node (noise3d, multiply... with its output type) to a Material Group's node view.
  - `group` (integer, required): The Material Group's layer id.
  - `node` (string, required): The MaterialX category, e.g. noise3d.
  - `type` (string): Its output type, e.g. color3 (the default) or float.
  - `x` (number): Canvas x.
  - `y` (number): Canvas y.
- **`operandi.nodes.connect`** — Wire a free node into <node>.<input> or layer:<id>.<channel>; the input's own value is kept but stops applying.
  - `group` (integer, required): The Material Group's layer id.
  - `from` (string, required): The free node.
  - `output` (string): Its output (out).
  - `to` (string, required): <node>.<input> or layer:<id>.<channel>, e.g. layer:4.diffuse_color.
- **`operandi.nodes.delete`** — Delete free nodes or layer boxes (layer:<id>) from a Material Group's node view; mixing nodes are refused.
  - `group` (integer, required): The Material Group's layer id.
  - `node` (string, required): Box names, comma-separated.
- **`operandi.nodes.disconnect`** — Remove the wire into <node>.<input> or layer:<id>.<channel>.
  - `group` (integer, required): The Material Group's layer id.
  - `to` (string, required): <node>.<input> or layer:<id>.<channel>.
- **`operandi.nodes.hide`** — Hide boxes in a Material Group's node view; renders don't change.
  - `group` (integer, required): The Material Group's layer id.
  - `node` (string, required): Box names, comma-separated.
- **`operandi.nodes.move`** — Place a box in a Material Group's node view.
  - `group` (integer, required): The Material Group's layer id.
  - `node` (string, required): The box.
  - `x` (number, required): Canvas x.
  - `y` (number, required): Canvas y.
- **`operandi.nodes.setValue`** — Set a free node's input value (a number, or comma-separated numbers).
  - `group` (integer, required): The Material Group's layer id.
  - `node` (string, required): The free node.
  - `input` (string, required): Its input.
  - `value` (any, required): A number or comma-separated numbers.
- **`operandi.nodes.show`** — Show hidden boxes in a Material Group's node view.
  - `group` (integer, required): The Material Group's layer id.
  - `node` (string, required): Box names, comma-separated.
- **`operandi.nodes.showAll`** — Show every hidden box in a Material Group's node view.
  - `group` (integer, required): The Material Group's layer id.
- **`select.editSet`** — Store the current selection as a named selection set.
  - `name` (string, required): Set name.
- **`select.useSet`** — Select a named selection set.
  - `name` (string, required): Set name.
  - `mode` (string): "replace" (default), "add" or "remove".
- **`select.deleteSet`** — Delete a selection set.
  - `name` (string): Set name; omit to delete every set of the current mode.
  - `all` (boolean): true deletes every set of the current mode.
- **`selset.ungroup`** — Delete a selection-set group (the sets stay).
  - `name` (string, required): Group name.
- **`select.convertGroup`** — Group selection sets.
  - `name` (string, required): Group name.
  - `sets` (array of strings): Set names; default every set of the current mode.
- **`vertMap.new`** — Add a weight map to the active meshes and select it (the one the Vertex Map Tools and the Vertex Map falloff use), or with type uv a UV map to the active mesh that becomes its selected UV map; one undo step.
  - `name` (string, required): The new map's name.
  - `type` (string): "weight" (the default) or "uv" (also "txuv").
  - `init` (boolean): Fill every point with value (0 when value is left out); false leaves the map empty. Default: filled only when value is given.
  - `color` (string): Accepted and ignored.
  - `value` (number): The weight every point gets, e.g. 0.5.
- **`select.vertexMap`** — Select the weight map the Vertex Map Tools and the Vertex Map falloff use (one at a time).
  - `name` (string): The map's name; left out selects none.
  - `type` (string): "weight" (default) or "wght".
  - `mode` (string): "set" (default) or "add" select it; "remove" selects none.
- **`scene.saveAs`** — Export the scene to an interchange file, as File > Export does (Mesh Stack results, hidden items as hidden). The document keeps its own file name and unsaved state, and nothing is added to the history. Here it only exports: it needs filename plus a format (format:, or a file extension that names one); to save the .opscene use scene_save.
  - `filename` (string, required): The file to write.
  - `format` (string): obj, stl, stlAscii, gltf, glb, geo, svg, lxo, ply, plyAscii, abc, usd, usda, usdc, usdz, fbx or 3mf; default from the file's extension (.stl and .ply mean binary). USD needs the running app's exporter (operandi-mcp --attach).
  - `export` (boolean): Accepted; here scene.saveAs always exports.
  - `animation` (string): USD: on (default; time samples once a frame over the scene range), off, or current (the current frame only).
- **`hide.sel`** — Hide the selected elements of the active mesh (everything when nothing is selected), one undo step; hidden polygons drop out of the UV tools. No arguments.
- **`hide.unsel`** — Hide the elements of the active mesh that aren't selected, one undo step. No arguments.
- **`hide.invert`** — Swap which elements of the active mesh are hidden, one undo step. No arguments.
- **`unhide`** — Show every hidden element of the selected meshes (every mesh when none is selected), one undo step. No arguments.
- **`operandi.render.addPerLightOutputs`** — Add one Light Path Expressions Render Output per renderable light, skipping lights that already have one. No arguments.
<!-- /command-args -->

### Commands made headless on 2026-11-11 (branch mcp-gaps)

- `poly.freeze`: bare uses the last successful Freeze's options (core
  `Editor::freeze_preset`), never the dialog.
- `scale.absolute`: values not given take the window's defaults (the
  window's own fields stay the app's); `mode:grab` is answered as a query
  (`{"value": "Grab Size: x × y × z m"}`) and changes nothing.
- `vertMap.new`, `select.vertexMap`.
- `operandi.nodeMaterial.import`; `operandi.nodeMaterial.setDefault`, with
  `value:"?"` answered like `item.channel`.
- `scene.saveAs` with a filename plus a format (`format:`, or an
  extension that names one) writes obj/stl/gltf/glb/geo/svg/lxo/ply/abc
  and USD files as File > Export does, from the scene as MeshOps evaluate
  it. It never changes the document's path or saved state and adds no
  history step. Without a filename, or for a plain `.opscene`, it refuses
  (use `scene_save`). USD needs the exporter the app registers: it works
  through `--attach`; the headless sidecar says so instead.
- `tool_run` takes `falloffs`: a Vertex Map (mode, index, mix) and Invert
  stack the tool runs under for that call only (the falloff from before
  comes back afterwards). The Add Falloff menu's `falloff.vertexMap` and
  `falloff.invert` are tool-starting entries in the registry, so they stay
  out of `command_run`.
- Not headless: `sculpt.liveWeightEdit`. Its setting belongs to the app's
  sculpt session (it only changes what a brush stroke updates), so a
  command run on the editor has nothing to change.

### Bridge tools waiting for registry commands

These bridge tools still do their own work because the command registry
has no command for it yet (plan L6 / UIC-160: the bridge should run
registry commands). They move onto the registry when these are added:

- `scene_import`: `scene.importReference` (filename, meshes, shaders,
  lights, environments, cameras) is in the name map
  (`specifications/command-names/MODO-NAMES-SUBSET.txt`) but not in the
  registry. Until then it reads the file with File > Import's reader and adds
  it through the Assembly preset load.
- `view_set viewport`: no registry command makes a viewport the active one.
  (`view_set shading` already runs the registry's `view3d.shadingStyle`.)
- `panel_show`: no registry command shows a panel or brings its tab to the
  front. The Window menu has no panel entries; the tab strip and its "+" menu
  change the panel groups directly.

### Atomic calls

Each call runs on a clone of the editor and is published only if every
command in it succeeds. A failed `tool_run` never leaves a tool armed, and a
failed `select` never leaves a half-made selection. Failures come back as
tool results with `isError: true` and the editor's own message (for example
"Extend supports open boundary edges"), not as protocol errors, so the agent
can read and react to them.

## What the first slice covers

Tested end to end through the message layer (`crates/operandi-mcp/tests`):
handshake and errors; cube → select → extrude → bevel with exact counts;
undo/redo restoring identical meshes; failed calls leaving state unchanged;
grow, convert, triple, rename, item transform, world-space readback; save and
reload.

Smoke-run on a cube (not yet in tests): sphere, cylinder, Transform, Move,
Rotate, Scale, Smooth Shift, Thicken, edge extrude/slide/crease/bevel/chamfer,
vertex bevel/extrude/merge/slide. Extend, Edge Relax and Tear correctly refuse
a closed cube; they need suitable fixtures.

Not exposed yet: Pen, Ellipsoid and Cone sessions (their one-shot
`primitive_add` is), Bridge, Spike, Polygon
Reduction, Chamfer Edit, Poly Haul, Sketch Extrude, Linear and Radial
falloffs (Vertex Map and Invert are `tool_run falloffs`), action center,
work plane, bevel presets, LXO import (`scene_import` refuses `.lxo`:
its import asks which groups to bring in), and multi-step interactive staging
(`Apply`/`Continue` within one tool session). Each is a few lines in
`bridge.rs` when a recipe needs it.

## Next

1. ~~**Attach to the running app.**~~ Done: see "Inside the running app".
2. **Interactive staging.** `tool_activate` / `tool_set` / `tool_apply` /
   `tool_drop` / `tool_cancel` for recipes that test live-session history.
3. **Stable identities.** Expose the identity runtime (`FaceHandle` etc.) so
   recipes can name elements independently of index order after edits.
4. **Scene digests.** A canonical, rounded mesh dump and hash for cheap
   result comparison on the research side.
