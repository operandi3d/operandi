# What's built, and how close it is to the reference

A summary for contributors and users, as of 2026-11-11. The day-by-day
build log is [PROGRESS.md](PROGRESS.md); known gaps in built tools are in
[TOOL_GAPS.md](TOOL_GAPS.md) and [SPECIFICATION_QUESTIONS.md](SPECIFICATION_QUESTIONS.md).

## Read this before the numbers

"Parity" here means: how much of the reference application's feature list
Operandi has. That needs a written list to count against, and the repository
only has one for part of the program.

- **Modelling toolbox: counted.** The tool-matrix spec
  (`specifications/tool-matrix-2026-11-05/`) lists all 243 buttons of the
  reference's Model toolbox. Each button was checked against Operandi's
  toolbox: it counts if Operandi has a working button for it, and doesn't if
  it's missing or shown greyed out as "not built yet".
- **A counted button isn't necessarily complete.** Many built tools still
  lack options, modes or gestures (see TOOL_GAPS.md), and some behaviour is
  Operandi's own *provisional* reading until a spec settles it. So the
  modelling figures are "the button exists and does its main job", which
  is an upper bound on true parity.
- **Every other area: not counted.** There's no reference feature list for
  UVs, materials, rendering, files and so on in the repository, so those
  areas get a plain description and a rough stage instead of a percentage.
  A percentage for them would be a guess.
- Nothing here comes from looking at the reference application; it all
  comes from the specs and Operandi's own code and tests.

## Modelling toolbox (counted)

| Tab | Reference buttons | Built in Operandi | Share |
|---|---|---|---|
| Basic (primitives, transforms, snapping, common tools) | 51 | 43 | 84% |
| Edge | 19 | 19 | 100% |
| Vertex | 18 | 17 | 94% |
| Polygon | 34 | 28 | 82% |
| Mesh Edit (slices, subdivide, booleans, drills, cleanup) | 25 | 17 | 68% |
| Curve | 25 | 8 | 32% |
| Deform | 24 | 7 | 29% |
| Duplicate | 47 | 7 | 15% |
| **All tabs** | **243** | **146** | **about 60%** |

The counts match labels in the code by hand and by script, so treat each one
as ±2. What's mostly missing:

- **Duplicate:** the Instance and Replica variants of Clone, Array, Scatter
  and Radial Array, all curve- and pen-driven duplication, Scatter and Mesh
  Paint. Mirror, Clone, Array, Radial Array and Radial Sweep are built.
- **Deform:** Flex, Bend, Vortex, Shear, Quantize, Jitter, Soft Drag/Move,
  Soft Selection tools, Element Push, Flare, Relax Mesh (greyed in the
  toolbox). Tweak, Push, Smooth, Twist, Swirl and Taper are built.
- **Curve:** curve offsets, conversions between curve types, Make
  Open/Closed Curves, hair guides, Retopo.
- **Mesh Edit:** Julienne and Dicer, Edge Zip, Fix Black Dots, Diagonal
  Re-Direct, retopology tools, Tack.
- **Basic/Polygon/Vertex:** Patch Curves, Axis Rotate, Element
  Snap, Vertex Normal tools, Topology Pen, Make Curve Fill, Triple Fan, Flip
  It, Vertex Tear.

## Other areas (not counted)

| Area | What's there | Stage |
|---|---|---|
| Selection | Point/edge/polygon/item modes, loops, rings, grow/shrink, between, paths, lasso, selection sets, symmetry, lock and hide | Broad |
| Shared tool systems | Action Center, falloffs (linear, radial), symmetry, snapping and precision presets, numeric fields with scrubbing, drop actions, per-tweak undo | Broad, some options provisional |
| Subdivision | Sub-D polygons, levels, edge weights, crease, freeze; evaluated display and picking | Usable |
| Sculpt and vertex maps | 11 brushes (Blender-derived maths), weight painting, vertex-map falloffs as masks | Started; masks pending |
| UVs | Unwrap and pack (Blender port), UV view, UV map list, Move/Rotate/Scale in UV, Fit, Flip, Mirror, Align, paint and lasso select, Checker | Started (editing stage 1) |
| Procedural (MeshOps) | A procedural stack on mesh items | Started |
| Items and scene | Meshes, groups, locators, cameras, lights, backdrops, environments, render item, hierarchy, multiple open scenes | Broad |
| Materials and shading | Layered shader tree (materials, gradients, procedurals, images, vertex maps, occlusion, group scale), node materials (first stage), material presets and a CC0 starter library, viewport per-pixel materials | Usable |
| Viewport | Shaded/wire/matcap styles, split and tear-off views, header buttons, quad view, navigation presets, 3D mouse and tablet input | Broad |
| Rendering | Cycles (CPU, Metal on macOS, denoising), render settings and outputs, preview render | Usable; GPU back ends other than Metal off |
| Files | Native `.opscene`; LXO import and export; OBJ, STL, glTF/GLB, PLY, VideoScape import and export; FBX import; Alembic import and export (polygon meshes); USD export (meshes, transforms, UVs, normals, Sub-D and creases); SVG export; MaterialX material import and export (OpenPBR, standard_surface, glTF PBR, UsdPreviewSurface; constants and image layers) | Usable; USD import and the later USD stages are still to come |
| Interface | Preferences, editable input map, colour themes, presets browser, floating palettes, dialogs, bug and crash reports; layouts (with their own key maps, palettes), a Form Editor and a menu bar made of forms (every menu editable), toolbox forms and user workbenches (edit in place, pick to add, tool-pipe presets) | Broad |
| Scripting | MCP bridge for agents and scripts; reference-style command names | Started; no Python API |
| Animation, rigging, dynamics, particles, texture painting, hair | — | Not started |

**Stages:** *Started* means the core works for the main case; *Usable* means
day-to-day work is possible with known gaps; *Broad* means most of what's
specified for the area is built.

## Deliberate look changes

- **Dispersion renders (2026-11-11, OpenPBR branch).** The Material's
  Dispersion was stored only; it now drives Cycles' dispersion on
  see-through glass, so older scenes that set it look different. Every
  other older material renders as before.

## How to update this page

When a toolbox button is built, change its tab's count and the total.
Recount from the tool-matrix spec when a new matrix or feature list arrives
(for example for shading or rendering); until then, describe those areas,
don't put a number on them.
