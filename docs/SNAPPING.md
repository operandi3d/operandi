# Snapping

Branch `snapping`, 2026-11-05, from `specifications/snapping-2026-11-05`
(supersedes the 3 px / 15 px range hint in the 2026-11-04 handle
preferences; the ranges are 24 / 40 px). Code: `operandi-app/src/snapping.rs`.

## What works

- **Switch (SN-01/02):** the Snapping button above the viewport, Edit >
  Snapping > Snapping, or **X** (input map "Snapping Toggle"). Off on a
  fresh install. Hold X through a drag to flip snapping for that drag only;
  letting go restores it. Toggling is not an undo step.
- **Snapping Options (SN-04/05/06/07):** Edit > Snapping > Snapping
  Options…, **F11** (opens under the pointer), or hold **Option** so the
  Snapping button reads "Options" and click it (right-click works too).
  Layout per Jason's QA: Presets menu (Default), the orange Snapping bar,
  Snap Types with a Mode dropdown (Global / Component / Items, each with
  its own ticks) and Preset (Default), then Grid, Vertex, Edge, Edge
  Center, Polygon, Polygon Center, Intersection, Box, Pivot, World Axis,
  Straight Line, Right Angle (only Grid and Vertex are built; the rest are
  greyed). Inner Range 24 px, Outer Range 40 px, Grid Distance 100 %.
  Coordinate Rounding (Fine), Fixed Increment (10 mm) and Snap Angle
  (15.0°) are shown greyed: not built yet. Saved per user in
  `snapping-preferences.json`, applied at once.
- **What snaps (SN-08/12):** the Move handle (axis arrows, the centre and
  off-model drags; also Transform's move), Tweak's grab point, and the
  centre a click off the model sets for Move, Rotate and Scale. Within the
  Outer Range the target gets a ring (theme colour "Snapping Target");
  within the Inner Range the handle jumps exactly onto it and the ring
  fills. Symmetry follows the snapped result.
- **Vertex targets (SN-09/10):** every point of every visible mesh, except
  the points being moved. Landings are exact and never weld (SNP-O-SN-03,
  both 3 px and 15 px short, pass).
- **Grid targets (SN-11):** crossings of the visible grid on the current
  work plane; the height off the plane is kept.
- **Typed values never snap (SN-03):** SNP-O-01 and SNP-O-02 pass.
- **Status (SN-13):** the Current Tool overlay line ends with "Snapping
  Vertex" (or the active targets) while snapping is on.

## Provisional (Operandi choices)

- Vertex targets on other meshes count. Points hidden behind a surface (as
  the drag began) are never targets, so a dense mesh never snaps through
  itself to its far side (Jason's QA, 11-05). Operandi choice.
- Grid Distance below 100 %: snaps only within that fraction of half a
  square of a crossing, on both axes.
- With both targets on, the nearer on screen wins; the grid always snaps
  at 100 %, so a vertex wins only when it is nearer than the nearest
  crossing.
- No stickiness: leaving the Inner Range releases the handle.
- Not yet: the Create panel's Draw tools' point placement (SN-12), named
  presets, and every target type other than Vertex and Grid.

## GUI checklist (QA app: Operandi snapping)

1. Make two cubes side by side (or one cube and move a copy). Vertex mode,
   select one corner, press W (Move).
2. Press X: the Snapping button reads "Snapping: Vertex" and the overlay
   line says so. Drag the handle toward a corner of the other cube: a ring
   appears near it, then fills and the point jumps onto it. Release: the
   point sits exactly on the other corner and the two stay separate.
3. Hold X and drag again: no snapping during that drag; after letting go
   of X, snapping is on again. Tap X: off.
4. F11 over the viewport: Snapping Options opens at the pointer. Tick Grid
   under Global, untick Vertex: dragging steps along grid crossings.
5. Type a Move offset into the tool panel near a target: it lands exactly
   where typed.
6. Tweak (Vertex mode): drag a point near another point: it snaps.
7. Quit and reopen: the switch and ticks come back as left.
