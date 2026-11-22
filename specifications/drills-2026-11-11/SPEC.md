# Solid Drill and Axis Drill: behaviour spec v1 (DRILL-xx)

Provenance: public user guide pages for Solid Drill, Axis Drill, Boolean and
Select by Previous Operation (names, options, intent); command argument lists
read through the scripting bridge (2026-11-11, read-only); four scripted Solid
Drill test runs from 2026-11-07 (cases D0-D3). Everything else is marked as our
own rule or an open question. REA-assisted: no.

Companion: the Booleans spec (Library `booleans/BOOLEANS.md`, BOOL-xx). Terms
**Target**, **Driver** and **Watertight** mean the same here. UI rules cite the
bible (`ui-conventions-bible/BIBLE.md`).

## Summary

Both tools cut the **target's own polygons** with a driver and never add driver
faces to the target. That's the difference from Boolean: no merging, no caps, no
walls. The target is usually a surface (a floor, a panel, a shell) and may be
open.

- **Solid Drill**: the driver is a closed solid. Target polygons are cut where
  the driver's surface passes through them, and each piece is either inside or
  outside the solid.
- **Axis Drill**: the driver is a flat outline (any polygons). Its shape is
  projected along X, Y or Z, like a cookie cutter pushed straight through
  everything, and target polygons are cut along that outline.

Both have the same four operations: Core, Tunnel, Stencil and Slice.

## Inputs

- **DRILL-01 Target.** The active mesh item(s) in the foreground. All its
  polygons take part (the tool ignores polygon selection, as Boolean does).
  Open, non-flat and multi-piece targets are all fine.
- **DRILL-02 Driver choice.** This is the same **Drive Mesh** option as BOOL-06:
  Background (default: visible, unselected meshes), Last Selected (selected
  polygons in the same item cut the rest; same rule as the Booleans Q1 answer)
  or Specific Surface Item (a mesh or a group, chosen with **Surface Item**,
  shown only in that mode). **Include Children** (default off) adds child
  items. Several drivers act as one combined driver (BOOL-07).
- **DRILL-03 Solid Drill driver.** It must be watertight. The driver pre-flight
  from Booleans applies to the driver only. Our rule: an open,
  self-intersecting or inside-out driver is refused with the Booleans notice
  (item name, problem, **Select Problem Edges**).
- **DRILL-04 Axis Drill driver.** Any face polygons. They needn't be closed,
  connected or flat. The cutter is the region covered by the driver polygons'
  shadows on the plane at right angles to **Axis**. Where driver polygons
  overlap, the region is their union. Our rule: curves, lines (two-point
  polygons) and lone points don't count. A driver whose shadow has no area (all
  its polygons lie edge-on to the axis) is refused: "The driver has no outline
  along Y."
- **DRILL-05 Driver untouched.** Driver geometry, materials, visibility and
  selection never change (BOOL-08).

## Options

| Option | Type | Default | Tools | Effect |
|---|---|---|---|---|
| Operation | choice | Core (our pick, Q4) | both | Core, Tunnel, Stencil, Slice (DRILL-10..13) |
| Axis | X / Y / Z | Y (our pick, Q3) | Axis Drill | direction of projection |
| Material | text | see Q5 | both | material for Stencil's inside pieces; ignored by other operations (shown greyed, not hidden, when Operation isn't Stencil; see Q8) |
| Drive Mesh | choice | Background | both | DRILL-02 |
| Surface Item | item or group | none | both | shown only for Specific Surface Item |
| Include Children | on/off | off | both | DRILL-02 |
| Use World Transform | on/off | on | procedural only | DRILL-30 |
| Select Sliced Edges | on/off | off | Axis Drill, procedural only | puts the new cut edges in an edge selection set |
| Edge Selection Set | text | empty | Axis Drill, procedural only | that set's name |
| Enable | on/off | on | procedural only | turns the operation off in the stack |

Select Sliced Edges, Edge Selection Set, Use World Transform and Enable are not
arguments of the one-shot commands, which is why they're procedural only.

- **DRILL-06 Command ids and arguments** (standard names, confirm with the
  command registry):
  - `poly.solidDrill mode:<int> material:<str> cutmesh:<int> item:<item> children:<bool>`
  - `poly.drill mode:<int> axis:<int> material:<str> cutmesh:<int> item:<item> children:<bool>`
  - `mode` and `axis` are required when scripted. `mode` is 0 Core, 1 Tunnel,
    2 Stencil, 3 Slice. That's the observed order for Solid Drill; Axis Drill is
    assumed the same (Q2). `axis` is 0 X, 1 Y, 2 Z. `cutmesh` is 0 Background,
    1 Last Selected, 2 Specific Surface Item.
  - Also accept readable values, as Boolean does: `core|tunnel|stencil|slice`,
    `x|y|z`, `background|lastSelected|item`.
  - Run with no arguments, each command opens its dialog (same pattern as
    `poly.boolean`).
- **DRILL-07 Dialog order.** The dialog lists Operation as Core, Stencil,
  Tunnel, Slice, the documentation's order, which differs from the numeric
  order. Q2 asks whether to keep that.

## Interaction

- **DRILL-08 One-shot dialog commands.** They sit in the Edit tab's
  "Booleans & Drill" group: **Solid Drill...** and **Axis Drill...**, labels
  ending in "..." (UIC-03). The dialog has OK and Cancel, and remembers its
  last values for the session. There's no viewport handle and no live preview
  in direct modelling.
- **DRILL-09 Long runs** show progress and can be cancelled with Esc. Cancel
  leaves the scene unchanged (BOOL-22).

## Result

Rules shared by all operations:

- **DRILL-14 Where the cut lines go.**
  - Solid Drill: a target polygon is cut wherever the driver's surface crosses
    it.
  - Axis Drill: a target polygon is cut by the outline's **prism**, the
    outline extended without end both ways along Axis. So front-facing,
    back-facing and edge-on polygons are all cut (answers Booleans Q2 our way).
  - Our design note: Axis Drill is exactly Solid Drill with that prism as the
    driver, so one cutting core serves both.
- **DRILL-15 Inside and outside.** After cutting, every piece of a target
  polygon is either inside the driver (inside the solid, or inside the prism)
  or outside it. A piece lying exactly on the driver's surface counts as
  inside (Q6).
- **DRILL-16 Polygons stay polygons** (BOOL-09).
  - Untouched polygons come through unchanged: same points, same order, same
    quads.
  - A cut polygon becomes one n-gon per piece. It isn't triangulated.
  - A piece with a hole in it (a hole cut fully inside one polygon) is split
    into the fewest hole-free polygons by joining the hole to the outer edge
    (two polygons for one hole; Q1).
  - No degenerate polygons, and no slivers from coplanar or coincident edges.
- **DRILL-17 Shared points.** New points on a cut line are shared by the
  pieces on both sides. Where the outline runs exactly along existing edges, no
  new points or edges are made.
  - Observed (D3): a 4x4 grid sliced by a cube whose outline lies on grid lines
    came back with the same 25 points and 16 quads.
- **DRILL-18 Attributes.** Materials, parts, smoothing and other per-polygon
  tags carry over to every piece. Per-point and per-corner maps (UVs, weights,
  colours, normals, morphs) are interpolated onto new points, and seams stay
  seams (BOOL-12, BOOL-13). Driver maps never come across, since no driver
  faces are added.

The four operations. The observed counts are for a 4x4 grid of unit quads
(-2..2 in X and Y, at Z 0) and a cube -1..1 covering its middle 2x2:

- **DRILL-10 Core.** Keep only the inside pieces; delete the rest. Observed
  (D0): 4 quads, 9 points.
- **DRILL-11 Tunnel.** Delete the inside pieces and keep the rest, leaving a
  hole. **No walls or caps are added**, so a closed target becomes open.
  Observed (D1): 12 quads, 24 points. The hole's rim points are shared, not
  duplicated.
- **DRILL-12 Stencil.** Keep everything, cut along the outline, and give the
  inside pieces the **Material**. Observed (D2): 16 quads, 4 of them with the
  new material. If the named material doesn't exist, it's created (Q5).
- **DRILL-13 Slice.** Like Stencil, but no material change. Observed (D3): 16
  quads.

Checks every run must pass. These are our stability rules (Jason: a failed
operation keeps the last good result and never crashes):

- **DRILL-19 Area checks.**
  - Slice and Stencil: total area is unchanged.
  - Core plus Tunnel areas: together they equal the original area.
  - The builder computes both, and a mismatch above tolerance (1e-6 relative)
    counts as a failure.
- **DRILL-20 Double classification.** Each piece's inside/outside answer comes
  from two independent tests. In Solid Drill they can be, for example, a
  winding-number test and the Booleans engine's own inside test; in Axis Drill,
  a 2D point-in-region test and the prism test. If the two disagree on any
  piece, it's a failure. This plays the same part as the Manifold + boolmesh
  cross-check in Booleans.
- **DRILL-21 Failure behaviour.** On any failure (pre-flight, a check,
  cancellation, an engine error, a panic caught at the command boundary), the
  scene stays exactly as it was, no undo step is added, and one notice says
  what went wrong. Never a half-applied result, never a crash.

## Selection after

- **DRILL-22** The tests left no polygon selection. Ours does the same and
  keeps the selection mode. The operation records named result sets for
  "select by previous operation" (BOOL-30):
  - Solid Drill: **Intersecting Edges** (the new cut edges), **Stencil
    Polygons** (the inside pieces), **Polygons from Geometry** (all result
    polygons).
  - Axis Drill: Intersecting Edges and Stencil Polygons.

## Refusals (status-line notice, nothing changes, no undo step)

- **DRILL-23** No driver qualifies: "No driver: show a background mesh or pick
  a Surface Item."
- **DRILL-24** The driver is not watertight (Solid Drill only), or its outline
  has no area (Axis Drill). See DRILL-03 and DRILL-04.
- **DRILL-25** The driver doesn't touch the target. Core would empty the
  target; the others would change nothing. Ours runs anyway:
  - Core leaves the target item empty but present, like Boolean's no-overlap
    Intersect (BOOL-17), with the note "Core: the driver doesn't reach the
    target, so nothing is left."
  - The other operations leave the target unchanged, with a note and no undo
    step.
- **DRILL-26** The target is the driver, in Background mode. That can't
  happen, since background means unselected. In Last Selected mode with nothing
  selected, apply DRILL-23.

## Undo

- **DRILL-27** One undo step per run, labelled "Undo Solid Drill Tunnel",
  "Undo Axis Drill Core" and so on. Undo restores the target exactly.

## Procedural (live) versions: a later stage

- **DRILL-30** Both tools also exist as mesh operations.
  - Adding one asks for the driver item first: **Drive Surface** for Solid
    Drill, **Template Surface** for Axis Drill. There is no background mode.
  - **Use World Transform** (default on): when on, target and driver are
    placed by their item transforms. When off, both are used in their own
    local coordinates as if both items sat at the origin.
  - The operation re-runs when the target, the driver or the driver's
    transform changes.
  - Stability rule: if a re-run fails (DRILL-21), the operation keeps
    outputting its **last good result** and shows a warning badge on the
    operation. It does not output an empty or broken mesh. Same stage as
    BOOL-32.

## Acceptance cases

In `cases.json`:

- **DR-D0..D3**: the observed grid cases, in direct modelling.
- **DR-S1..S4**: a single big quad with a hole cut inside it. These are our
  rules; check polygon counts against Q1.
- **DR-S5**: a closed cube tunnelled by a smaller cube through two faces. The
  result is open, with no walls.
- **DR-A1..A4**: Axis Drill on a cube with a small square outline along Y.
  Back faces are cut, and the side walls are classified by the prism.
- **DR-A5**: overlapping driver polygons cut as one outline.
- **DR-F1..F3**: refusals, with the scene unchanged.

## Open questions (with the default we build until answered)

Q1 is a scripted test away; Q2 to Q8 are design calls, with defaults.

- **Q1** Does Modo's Tunnel/Stencil split a polygon with an interior hole the
  same way as DRILL-16? Default: two hole-free polygons. One scripted probe
  would answer it, with Jason's go.
- **Q2** Axis Drill's numeric mode order, and the dialog order. Default:
  numbers as Solid Drill (0 Core, 1 Tunnel, 2 Stencil, 3 Slice); the dialog
  lists Core, Stencil, Tunnel, Slice as the docs do.
- **Q3** Axis default. Default: Y (drilling down into a floor).
- **Q4** Operation default. Default: Core (first in the list).
- **Q5** Stencil with an empty Material field. Default: the dialog's Material
  starts as "Stencil"; an empty value uses a material named "Stencil", created
  if missing.
- **Q6** Pieces exactly on the driver surface. Default: inside.
- **Q7** Does Axis Drill cut polygons lying edge-on to the axis (walls)?
  Default: yes, by the prism (DRILL-14).
- **Q8** Should Material be greyed or hidden when Operation isn't Stencil?
  Default: greyed, per the bible's rule for a field that depends on another
  choice in the same dialog. The builder cites the rule number.
