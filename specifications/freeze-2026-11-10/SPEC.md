# Freeze (curves and the Freeze dialog) — behavior spec v2

Provenance: observed in Modo 17.1v1 through scripted probes (2026-11-11,
two batches, 30 cases); user guide sections "Freezing Geometry", "Freeze for
Procedural Modeling" and "Mesh Item" (Curve tab) for names, options and
intent; the 36-segments-per-span sampling first seen in DRW-11.
REA-assisted: no.

This spec extends two earlier ones and does not replace them:
SDV-12/13/15 (subdivision Freeze, in the subdivision spec) and SUB-50/50a
(the Freeze... dialog, in the toolbox panels Edit spec). Rules here are
numbered **FRZ-nn**. *(observed)* means seen in the reference;
*(build guess)* means neither the guide nor an observation settles it.

v2 changes from v1: selection limits what freezes (FRZ-01); Tessellation off
now leaves curves alone too (FRZ-07); kept points (FRZ-04); equal parameter
steps confirmed (FRZ-05); Fixed Spans off is a decision (FRZ-06); no
refusal message in the reference (FRZ-09).

## Summary

Freeze turns "live" geometry into plain, editable geometry, fixed in the
shape the viewport shows right now. Operandi already freezes subdivision
surfaces. This version adds **curves** (Curve, B-Spline and Bezier line
elements): they become ordinary straight-edged geometry, either one Face
polygon or one straight polyline each. A mesh holding both curves and
subdivision surfaces freezes in one step. Displacement, deformers, morph
maps and hole-drilling stay out of scope until Operandi has those features.

## Inputs

- **FRZ-01 Scope** *(observed)*.
  - Item mode: every curve (and subdivision polygon, SDV-12) in the active
    mesh freezes.
  - Polygon mode with polygons selected: only the selected curves and
    polygons freeze; unselected curves stay curves.
  - Vertex mode with points selected: nothing freezes.
  - Polygon mode with nothing selected, and Edge mode: not observed; treat
    them like Item mode *(build guess, FRZ-Q1)*.
- **FRZ-02 What counts as a curve** *(observed)*: a line element drawn as a
  Curve, B-Spline or Bezier. A plain straight polyline is left unchanged,
  whatever Freeze Curves as says. Face polygons are never changed.

## Options

All ten SUB-50 fields keep their places, defaults and memory. This version
makes four of them act; the other six stay greyed with "Not available yet".

| Field | Kind | Default | Acts in v2 | Effect |
|---|---|---|---|---|
| Freeze Curves as | 2 Point Polygons / Faces / Polylines | Faces | yes | form of each frozen curve (FRZ-04) |
| Make Holes | check | off | no | — |
| Drill Axis | X / Y / Z | Y | no | greyed while Make Holes is off |
| Freeze Displacement | check | on | no | — |
| Freeze Deformation | check | on | no | — |
| Tessellation | check | on | yes | off: nothing changes, curves included (FRZ-07) |
| Subdivide Curves with Fixed Spans | check | on | yes | on: frozen curve matches its viewport drawing (FRZ-05) |
| Curve Refinement Angle | angle, > 0 | 5° | yes | used only when Fixed Spans is off (FRZ-06); greyed while Fixed Spans is on |
| Save Deformation to Morph | check | off | no | — |
| Morph Map Name | text | empty | no | greyed while Save Deformation to Morph is off |

Curve Refinement Angle accepts values above 0° and at most 180°; out-of-range
input is clamped, per the bible's number-field rules *(build guess)*.

## Interaction

- **FRZ-03** Freeze... opens the dialog; OK freezes with the shown values and
  remembers them; Cancel or closing changes nothing (as SUB-50). The plain
  Freeze action (menu, toolbar, typed command) runs immediately with the
  remembered values, no dialog.

## Result

- **FRZ-04 The forms.** Each frozen curve is turned into a chain of samples
  along its drawn shape (FRZ-05/06), then:
  - **Faces** *(observed)*: one Face polygon through all the samples in
    order. An open curve simply closes from its last sample back to its
    first. This happens even when the samples are collinear (a straight
    Curve gives a flat, zero-area Face).
  - **Polylines** *(observed)*: one straight polyline through the samples,
    open like the source curve.
  - **2 Point Polygons** *(build guess, FRZ-Q2)*: one two-point line polygon
    per segment, neighbours sharing their point. The reference's scripted
    form has no value for this choice, so it was not observed.
  - **Points** *(observed)*: curve points that lie on the drawn curve are
    kept and reused as samples (both ends of a Bezier, and every point of a
    Curve that passes through its points). Bezier handle points are removed.
    A kept point that other polygons also use stays shared with them.
    B-Spline points (which the curve does not pass through) are removed
    unless used elsewhere *(build guess)*.
  - Closed curves: as above, with the closing span sampled too and the
    polyline closed *(build guess, not observed)*.
- **FRZ-05 Fixed Spans on** *(observed)*. Each span is cut into 36 steps,
  equal in the curve's own parameter (not in length), so the result lies on
  the displayed curve. Every span gets 36, straight or curved. Example: the
  open Bezier with anchors (0,0,0), (3,1,0) and handles (1,1,0), (2,0,0)
  freezes to 37 samples; the second sample is (0.083333, 0.078789, 0) and the
  middle one (1.5, 0.5, 0).
- **FRZ-06 Fixed Spans off — DECISION NEEDED.** The guide says spans are cut
  adaptively by Curve Refinement Angle (tighter bends, more segments). The
  reference, at every angle tried (ten values from 0 to 200, read as degrees
  or as radians), cut each span into a
  single straight segment (a one-span Bezier became one two-point polygon).
  - Option A (recommended): follow the guide. Each span gets the fewest
    equal-parameter steps such that the direction turns by no more than the
    angle between neighbouring segments; a straight span gets one. Operandi
    choice.
  - Option B: copy the reference: one segment per span, angle ignored.
  Until decided, build Option A.
- **FRZ-07 Tessellation** *(observed)*. On: subdivision polygons freeze
  (SDV-12/13/15) and curves freeze (FRZ-04..06), together in one step for a
  mixed mesh. Off: nothing changes, neither the subdivision cage nor the
  curves. (This differs from v1, which guessed curves would still freeze.)
- **FRZ-08** Positions of everything not frozen are unchanged. UVs and
  weights on frozen subdivision polygons follow SDV-15 / EW-10; frozen
  curves carry none *(build guess)*.

## Selection after

No polygons are selected after Freeze in Polygon mode *(observed)*. In
Vertex mode, where nothing freezes, the point selection stays.

## Refusals

- **FRZ-09** Nothing to freeze (no curves or subdivision polygons in scope,
  or Tessellation off): nothing changes and no undo step is added. The
  reference says nothing *(observed)*; Operandi says so in the status line,
  for example "Nothing to freeze" *(Operandi choice)*. This replaces today's
  "Freeze doesn't handle meshes with lines yet" refusal.
- No visible mesh to act on: the existing "Select a visible mesh to freeze".

## Undo

- **FRZ-10** One undo step named "Freeze" for the whole operation *(build
  guess: undo can't be observed by script)*. Undo restores the curves, the
  cage and the selection.

## Command

The Freeze command already in Operandi's registry gains one argument per
dialog field, in the dialog's order; the exact argument names come through
the command registry. In the reference's scripted form the curves choice has
two values only, 0 = Faces and 1 = Polylines *(observed)*; any other value
leaves the curve in place and adds stray points, which Operandi must not
copy: Operandi uses 0 = Faces, 1 = Polylines, 2 = 2 Point Polygons and
rejects other values. Drill Axis 0/1/2 = X/Y/Z. Arguments for fields that
don't act yet are accepted and ignored, with a status note.

## Acceptance cases

See `cases.json`: FRZ-C01..C13 (forms, sample positions, kept points,
selection scope, Tessellation off, mixed mesh, nothing to freeze, Fixed
Spans off under Option A, undo).

## Open questions

- **FRZ-Q1** Polygon mode with nothing selected, and Edge mode with edges
  selected: what freezes?
- **FRZ-Q2** What the dialog's 2 Point Polygons choice produces (needs the
  dialog, hands-on).
- **FRZ-Q3** Closed curves (not built in the probes).
- **FRZ-Q4** Whether 36 per span changes with the mesh's curve display
  setting; no such setting was found on the mesh item by script.
- **FRZ-Q5** FRZ-06 decision (Option A or B).
- Out of scope, listed for later: Make Holes / Drill Axis, Freeze
  Displacement, Freeze Deformation, Save Deformation to Morph, spline
  patches, the Mesh Operations Freeze / Duplicate and Freeze, Freeze
  Replicator / Freeze Surface Generator, and the item Properties transform
  Freeze.
