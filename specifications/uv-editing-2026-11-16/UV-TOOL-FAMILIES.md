# UV tool families: one engine, many buttons (v1, 2026-11-16)

Behaviour-only. REA-assisted: no. No bridge pass (the bridge was busy with
other threads). Follows the TOOL FAMILIES rule in
`tool-families-2026-11-15/TOOL-FAMILIES.md`: build each engine once, and
every button is a cheap preset on top of it.

**Sources.** (1) The reference's public user guide, UV chapter (local copy,
paraphrased). (2) The reference's plain-text toolbox and tool-preset
settings, which give the UV tab's sections, button order and which buttons
are presets of which tool. (3) Operandi main `6363915d`, read for status.
(4) Earlier specs: `uv-unwrap-pack-relax-2026-11-11` v1.1 (here called
**UVS-1**) and `tutorial-gaps-2026-11-13/SPEC-UV-EDITING.md` v1
(**UVE-1**). The new behaviour for everything else is in
`BEHAVIOR-UV-TOOLS.md` in this folder (**UVT**).

Status words: **Built**, **Partly**, **Missing**.

---

## 0. What's already in Operandi

| Piece | Status | Where |
|---|---|---|
| UV engine crate: chart building, Conformal (least-squares conformal) and Angle Based (ABF++) solvers, minimum-stretch (SLIM) solver, shape packer, box packer | Built | `operandi-uv` (Blender port, GPL-2.0-or-later; SLIM solver file MPL-2.0) |
| UV Unwrap | Partly (one-shot dialog, not the live tool) | `uv.unwrap` |
| Pack UVs | Partly (Normalized, UDIM, Manual, Tiles; no Nearest, Polygon Tag, By Item, Background, New Map) | `uv.pack` |
| UV view | Partly (grid, seams, overlap, distortion, click select, island select; no image, checker, coverage, paint or lasso select) | View > UV View |
| UV maps stored per mesh, many named maps | Built (storage) | mesh `uv_maps` |
| UV map list (new, delete, rename, copy, paste) | Missing | |
| Everything else in this file | Missing | |

The engine crate already holds most of the hard maths. What's missing is
mostly workflow: tools, the map list, and running the existing transform
pipe in UV space.

---

## 1. The reference's UV tab, as laid out

The UV tab of the toolbox has a **Map** picker at the top, then these
sections in this order. Each row says which engine it is, and whether it
is a plain engine button or a preset.

### 1.1 Create

| Button | Engine | Kind | Spec | Status |
|---|---|---|---|---|
| Project | E2 Project | engine | UVT §2 | Missing |
| Project from View | E2 Project (view frame) | preset | UVE-1 §4, UVT §2.4 | Missing |
| Unwrap | E3 Unwrap | engine | UVS-1 §1 | Partly |
| Cut Map... (popover) | E13 Seams | panel | UVS-1 §1.6, UVT §13 | Missing |
| Relax | E4 Relax | engine | UVS-1 §3 | Missing |
| Rectangle | E5 Grid layout | engine (one-shot) | UVT §5.2 | Missing |
| Peeler | E5 Grid layout | engine (tool) | UVT §5.1 | Missing |

Inside Cut Map, the reference's own preset settings show:
- **Unwrap Tool** = Unwrap with Seam Source set to the seam map.
- **Unwrap & Relax** = a separate combined tool, with Seam Source set to
  the seam map. Operandi should build it as Unwrap followed by Relax on
  the same pipe (E3 then E4), not as a third solver.
- **Seam Marker** = the Conditional Loop selection tool with its action
  set to "Seam Map". So it is a preset of a selection engine, not a UV
  engine (UVT §13.2). Conditional Loop isn't built in Operandi yet either.

### 1.2 Tools (Transform Tools)

| Button | Engine | Kind | Status |
|---|---|---|---|
| Box Transform | E6 UV transform, box handle | engine variant | Missing |
| Move, Rotate, Scale | E6 = Operandi's Transform tool run in UV space | preset | Missing (UVE-1 §2) |
| Element Move | E6 | preset | Missing |
| Warp: Smooth, Jitter, Shear, Taper, Pole | E6 + falloff, the same presets as in 3D | preset | Missing in UV (Built in 3D) |

### 1.3 Align

| Button | Engine | Kind | Status |
|---|---|---|---|
| Axis Align: Up, Down, Left, Right | E7 Island affine (flatten to bound) | preset | Missing (UVE-1 §6) |
| Quick Axis Align: Auto, Horizontal, Vertical | E7 (rotate island to edge) | preset | Missing |
| Island Align (4 × 4 grid of U and V choices) | E7 (move island to group bound) | preset | Missing |

### 1.4 Align and Pack

| Button | Engine | Kind | Status |
|---|---|---|---|
| Orient Pieces (Auto) | E7 (rotate each island) | preset | Missing |
| Pack... | E8 Pack | engine | Partly |
| Fit UVs | E7 (scale to tile) | preset | Missing (UVE-1 §3) |
| Texel Density (sub-panel) | E7 (scale islands to a density) | preset group | Missing |

### 1.5 Flip, Mirror, Align

| Button | Engine | Kind | Status |
|---|---|---|---|
| Flip U, Flip V, With Options | E7 (mirror about own bound centre) | preset | Missing |
| Mirror U, Mirror V, With Options | E7 (mirror about a set line) | preset | Missing (UVE-1 §5) |
| Radial Align | E6: the Radial Align vertex tool run in UV space | preset | Missing (also unbuilt in 3D; build once, works in both) |
| Linear Align | E6: the Linear Align vertex tool run in UV space | preset | Missing (also unbuilt in 3D; build once, works in both) |

### 1.6 Split and Sew

| Button | Engine | Kind | Status |
|---|---|---|---|
| Split and Sew Tool (brush) | E9 Cut and join | engine (brush) | Missing |
| Split | E9 | preset | Missing |
| Sew | E9 | preset | Missing |
| Move and Sew | E9 + E7 (place island, then join) | preset | Missing |

### 1.7 Symmetry

| Button | Engine | Kind | Status |
|---|---|---|---|
| UV Symmetry (Off, U, V) | E10 UV symmetry pipe layer | layer | Missing |
| Symmetrize... | E10 + 3D symmetry pairing | engine (one-shot) | Missing |
| Center | E9 (join at the symmetry line) | preset | Missing |

### 1.8 Utilities

| Button | Engine | Kind | Status |
|---|---|---|---|
| Select Error UVs... | E11 UV queries | engine | Missing |
| UDIM Indicator | E14 Display, with E7 for its nudge arrows | tool | Missing |
| Select: Border, Invert, Convert, More, Error, Unmapped | E11 (Invert, Convert, More are Operandi's existing select commands) | preset | Missing (Border, Error, Unmapped) |
| Edit UVs: Cut, Copy, Paste (Selection or Connectivity), Merge UV Maps, Transfer, Delete, Set Value, Rotate Polygons | E12 UV values | engine group | Missing |
| Export: EPS, SVG, Convert UVs to Mesh | E15 Export | engine group | Missing |
| Real World Size (1 UV = …, Apply Overall) | E7 texel density | preset | Missing |

---

## 2. The engines

| # | Engine | What it really does | Buttons it covers | Borrowed maths |
|---|---|---|---|---|
| E1 | **UV map store and list** | Named maps per mesh, one current; new, delete, rename, copy, paste, merge | Map picker, Lists UV Maps, Merge UV Maps | none |
| E2 | **Project** | Writes UVs from a projection: Planar, Cylindrical, Spherical, Cubic, Barycentric, Atlas, Atlas 2, View | Project, Project from View, Unwrap's Initial Projection | xatlas charting for Atlas and Atlas 2 only (Jason 11-16) |
| E3 | **Unwrap** | Cut along seams, flatten each island, lay out | Unwrap, Unwrap & Relax (first half), Cut Map's Unwrap buttons | Built (Blender port) |
| E4 | **Relax** | Re-solve an existing map: Spring, Conformal, Angle Based, Adaptive; pins; border rules | Relax, Unwrap & Relax (second half) | Blender's minimum stretch and solvers (built); spring and border straighten are our own |
| E5 | **Grid layout** | Lays quad strips or quad patches onto a regular grid | Peeler, Rectangle, Relax's Straighten border | our own (simple) |
| E6 | **UV transform** | Operandi's existing Transform pipe (action centre, falloff, symmetry, snapping, then tool) with U and V instead of X, Y, Z | Move, Rotate, Scale, Transform, Element Move, Warp presets, Box Transform, Radial Align, Linear Align | none |
| E7 | **Island affine** | Groups UVs (all, per island, per polygon, per edge row), works out one 2D move, turn, scale or flip per group from a rule, and applies it | Fit, Flip, Mirror, Axis Align, Quick Align, Island Align, Orient Pieces, To Origin, Texel Density, Real World Size, UDIM nudge | Blender's minimum-area box turn (built, for Orient) |
| E8 | **Pack** | Island layout without overlap | Pack, Rectangle's Pack option, Unwrap's layout | Built (Blender port) |
| E9 | **Cut and join** | Splits shared UV points apart or welds matching ones together | Split, Sew, Move and Sew, Split and Sew brush, Center | none |
| E10 | **UV symmetry** | A pipe layer that mirrors edits across U or V; plus Symmetrize, which copies UVs between 3D mirror polygons | UV Symmetry, Symmetrize | Operandi's symmetry pairing (built) |
| E11 | **UV queries** | Finds UV problems and selects them | Select Border, Error UVs, Unmapped, region select | Blender packer's overlap tests can be reused |
| E12 | **UV values** | Clipboard and direct edits of UV values | Cut, Copy, Paste, Transfer, Delete, Set Value, Rotate Polygons | none |
| E13 | **Seams** | Seam maps and seam display | Cut Map popover, Seam Marker (Conditional Loop preset) | none |
| E14 | **UV display** | The UV view and its overlays | UV view, checker, distortion, overlap, flipped, coverage, UDIM frame | none |
| E15 | **Export** | UV layout to EPS or SVG; UVs to a flat mesh | Export EPS, SVG, Convert UVs to Mesh | none |

**The big saving is E6 plus E7.** About 35 of the roughly 60 UV buttons are
E6 or E7 presets. E6 costs one change: let the existing tool pipe edit the
current UV map in a UV view. Every 3D transform preset, falloff and action
centre then works in UV space for free. E7 is one small engine with a
grouping choice and a rule table, and each button is one row of that table.

---

## 3. How the tool pipe applies in UV space

The reference treats the UV view as one more viewport: most 3D tools work
in it on U and V. For Operandi:

| Pipe layer | In a UV view | Notes |
|---|---|---|
| Action centre | Yes. Selection centre by default; Pivot and Origin mean UV (0, 0) and the box pivot | UVT §6 |
| Falloff | Yes for E6 and Relax (the guide says falloffs can limit Relax). Linear and Radial measured in UV space | UVT §6.3 |
| Symmetry | The **UV Symmetry** layer (Off, U, V, with a centre offset) replaces 3D symmetry in UV views. 3D symmetry stays the rule in 3D views | UVT §10 |
| Snapping | Grid and UV points. Box Transform's Shift snap to the main grid lines | UVT §6.2, question UVT-Q05 |
| Tool | U, V edits written into the current map only. 3D positions never move | |

From a **3D view**, UV tools that need a click (Project from View, Unwrap
drags, Relax drags) work too. UV transforms driven from a 3D view are later
work (UVE-1 Q7).

---

## 4. Counting it up

| | Engines | Buttons |
|---|---|---|
| Built or partly built | E3, E8, part of E14, E1 storage | 3 |
| Missing, maths already in the crate | E4 | 2 |
| Missing, cheap (workflow only) | E1 list, E6, E7, E9, E11, E12, E13 | about 45 |
| Missing, new maths | E2 Atlas (xatlas), E5 | 4 |
| Missing, low value | E15, Texel image-size helpers | 6 |
