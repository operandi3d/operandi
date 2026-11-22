# Preferences: Display > Colors (behaviour spec v1, 2026-11-04)

Behaviour-only. Values are the reference application's factory defaults.
Hex = float × 255, rounded; whether the reference treats these floats as
display values or linear values is still being checked (COL-Q1), so treat
the floats as the source of truth.

## Panel behaviour

- One scrolling list of colour swatches in the order below, separated by
  dividers; some dividers have a heading (Handles, Text, Selection
  Rollovers, Topo, PSUB Masks, OpenGL Highlights).
- At the top: **Load from Scheme** (dropdown of saved schemes) and **Save to
  Scheme...** (asks for a name, saves the current swatches as a scheme).
- Each 3D view draws with a named scheme. A scheme stores only the colours
  it changes; everything else comes from the base set below. Editing a
  swatch changes the base set; to restyle a view, save a scheme and pick it
  for that view (each view keeps its own scheme).
- Locked Mesh Elements appears twice in the list (row 45); both
  rows edit the same colour.
- The three Work Plane axis colours (rows 6–8) have no labels and follow
  the Work Plane row.

## Colours

"Default view" is what the stock perspective views draw: they use a stock
scheme that overrides six entries. Relevance: now / later / never for
Operandi. "Build now": can be wired to an existing Operandi feature today.

| # | Section | Setting | Base default | Default view | What it colours | Relevance | Build now |
|---|---|---|---|---|---|---|---|
| 1 | Background and grids | Background | 0.36 0.40 0.45 #5C6673 | 0.22 grey #383838 | Viewport background when the view's background mode is None or Gradient | now | yes |
| 2 | Background and grids | Deformers Active | 0.36 0.40 0.42 #5C666B | 0.19 grey #303030 | Background instead of #1 while deformers are switched on in the view | later | no |
| 3 | Background and grids | Setup Mode | 0.70 0.70 0.20 #B2B233 | — | Background tint while rig Setup mode is on (inferred from the name; unconfirmed) | never | no |
| 4 | Background and grids | Grid | 0.00 0.00 0.00 #000000 | 0.52 grey #858585 | The ground-plane grid lines | now | yes |
| 5 | Background and grids | Work Plane | 0.56 0.60 0.65 #8F99A6 | 0.36 0.54 0.93 #5C8AED | The work (construction) plane grid | now | yes |
| 6 | Background and grids | (Work Plane X) | 1.00 0.30 0.20 #FF4C33 | — | Work plane's X axis line | now | yes |
| 7 | Background and grids | (Work Plane Y) | 0.30 1.00 0.40 #4CFF66 | — | Work plane's Y axis line | now | yes |
| 8 | Background and grids | (Work Plane Z) | 0.30 0.40 1.00 #4C66FF | — | Work plane's Z axis line | now | yes |
| 9 | Geometry | Wireframe | 0.72 grey #B8B8B8 | same | Uniform wireframe colour; items can override it | now | yes |
| 10 | Geometry | Color Wireframe | 0.00 0.00 0.00 #000000 | same | Wire colour when an item uses its own colour-wireframe mode | later | no |
| 11 | Geometry | Background Item | 0.00 0.00 0.00 #000000 | — | Inactive (unselected, background) meshes | now | yes |
| 12 | Geometry | Instance | 0.80 0.60 1.00 #CC99FF | — | Instanced copies of an item | later | no |
| 13 | Geometry | Solid | 0.60 grey #999999 | — | Surface colour in the Solid shading style | now | yes |
| 14 | Geometry | Patch Cage | 0.28 0.77 0.40 #47C466 | — | Control cage drawn around subdivision surfaces | now | yes |
| 15 | Geometry | Discontinuous Pair | 0.60 0.60 1.00 #9999FF | — | Matching discontinuous UV edges in the UV view | later | no |
| 16 | Selection | Selection | 1.00 0.66 0.16 #FFA829 | — | Selected points/edges/polygons and items (outline) | now | yes |
| 17 | Selection | Selection Fill | 1.00 0.66 0.16 #FFA829 | — | Fill of selected polygons (drawn translucent) | now | yes |
| 18 | Selection | Child Highlight | 0.55 0.61 0.68 #8C9CAD | — | Children of a selected parent item | later | no |
| 19 | Selection | Assembly Highlight | 0.50 0.85 0.70 #80D9B2 | — | Hovered assemblies | never | no |
| 20 | Selection | Lasso | 0.40 1.00 1.00 #66FFFF | — | Lasso / drag-select outline | now | yes |
| 21 | Handles | Handle | 0.40 1.00 1.00 #66FFFF | — | Tool handle parts not under the pointer | now | yes |
| 22 | Handles | Active | 1.00 0.90 0.40 #FFE666 | — | Handle part under the pointer / being dragged | now | yes |
| 23 | Handles | Axis X | 0.90 0.20 0.20 #E63333 | — | X-constrained handle parts | now | yes |
| 24 | Handles | Y | 0.20 0.80 0.20 #33CC33 | — | Y-constrained handle parts | now | yes |
| 25 | Handles | Z | 0.20 0.40 1.00 #3366FF | — | Z-constrained handle parts | now | yes |
| 26 | Handles | Guide | 0.80 0.60 1.00 #CC99FF | — | Snapping guides | later | no |
| 27 | Handles | Cage | 0.40 0.80 0.60 #66CC99 | — | Curve/spline cage edges | later | no |
| 28 | Handles | Label | 0.80 grey #CCCCCC | — | Item labels drawn in the view | later | no |
| 29 | Handles | Range Cool | 0.20 0.00 0.65 #3300A6 | — | Low-weight end of falloff display | now | yes, if falloff shading is drawn |
| 30 | Handles | Range Warm | 1.00 0.85 0.00 #FFD900 | — | Full-weight end of falloff display | now | yes, if falloff shading is drawn |
| 31 | Handles | Unsnapped | 0.90 0.70 1.00 #E6B2FF | — | Pre-highlight of a nearby snap target | now | after snapping highlight exists |
| 32 | Handles | Auto Haul Drop Shadow | 0.20 grey #333333 | — | Shadow behind on-screen auto-haul sliders | later | no |
| 33 | Text | Selection | 1.00 0.78 0.28 #FFC747 | — | On-screen text about the selection | now | yes, if a viewport info overlay exists |
| 34 | Text | Info | 0.76 0.80 0.85 #C2CCD9 | — | Other on-screen info text | now | yes, if a viewport info overlay exists |
| 35 | Text | Weight Values | 0.74 0.82 0.74 #BDD1BD | — | Numbers shown when weight values are displayed | later | no |
| 36 | Selection Rollovers | Generic | 0.55 0.71 0.78 #8CB5C7 | — | Pre-highlight of the element under the pointer | now | yes |
| 37 | Selection Rollovers | Face | 0.55 0.71 0.78 #8CB5C7 | — | Pre-highlight on face (non-subdivided) polygons | now | yes |
| 38 | Selection Rollovers | Subdivision | 0.70 0.70 0.80 #B2B2CC | — | Pre-highlight on subdivision polygons | now | yes |
| 39 | Selection Rollovers | Catmull-Clark | 0.70 0.80 0.70 #B2CCB2 | — | Pre-highlight on Catmull-Clark polygons | now | yes |
| 40 | Topo | Topo Face | 0.20 grey #333333 | — | Polygons in Topo display mode | later | no |
| 41 | Topo | Topo Vert | 0.38 0.62 0.92 #619EEB | — | Points in Topo display mode | later | no |
| 42 | Topo | Topo Edge | 0.11 0.25 0.41 #1C4069 | — | Edges in Topo display mode | later | no |
| 43 | Topo | Topo Back Edge | 0.11 0.25 0.41 #1C4069 | — | Back-facing edges in Topo display mode | later | no |
| 44 | PSUB Masks | PSUB Mask | 1.00 0.30 0.30 #FF4C4C | — | Sculpt mask overlay | never | no |
| 45 | PSUB Masks / OpenGL Highlights | Locked Mesh Elements | 0.329 0.580 0.380 #549461 | — | Locked (protected) points/polygons | later | no |
| 46 | OpenGL Highlights | UV Boundary | 0.65 0.15 0.93 #A626ED | — | UV island borders in 3D views (Show UV Boundaries) | now | yes |
| 47 | OpenGL Highlights | Geometry Boundary | 0.12 0.89 0.57 #1FE391 | — | Open (border) edges of meshes (Show Geometry Boundaries) | now | yes |
| 48 | OpenGL Highlights | SelOp Selection | 0.00 1.00 0.256 #00FF41 | — | Selections made by procedural selection operations | never | no |
| 49 | OpenGL Highlights | UV Seam | 0.60 0.60 1.00 #9999FF | — | UV seams drawn in 3D views | now | yes |
Row 45 is listed in both the PSUB Masks and OpenGL Highlights sections.

Other base colours not on this panel: ghosted mesh 0.39 0.58 0.93,
morphed mesh surface 0.62 0.63 0.68, wire 0.71 0.72 0.78, vectors 0 1 1
(later); warning 1 1 0.2 and alert 1 0.2 0.2 (never).

## Stock schemes

| Scheme | Used by | Overrides |
|---|---|---|
| Perspective (stock) | default perspective 3D views | Background #383838, Deformers Active #303030, Work Plane #5C8AED, Grid #858585, Wireframe #B8B8B8, Color Wireframe #000000 |
| Orthographic (stock) | default top/front/side views | Background #393939, Deformers Active #434343, Work Plane #8F99A6, Grid #000000, Wireframe #B8B8B8, Color Wireframe #000000 |
| UV (stock) | default UV view | Background #434343, Deformers Active #434343, Work Plane #262626, Grid #000000, Wireframe #B0B2B2, Color Wireframe #000000, Axis X #B2991A, Axis Y #1A99B2 |

Several further optional schemes exist (darker, neutral, blue and slate
variants, and look-alikes of other applications); the look-alikes are
never.

Minimum useful behaviour for Operandi: one base colour set, per-view
schemes with partial overrides (three stock ones above), and Save/Load
scheme.

## Open questions

- COL-Q1: are stored floats display values or linear?
- COL-Q2: does editing a swatch leave open views unchanged until a scheme
  is saved and applied (as documented)?
- COL-Q3: exact layout of the three unlabeled Work Plane axis swatches.
