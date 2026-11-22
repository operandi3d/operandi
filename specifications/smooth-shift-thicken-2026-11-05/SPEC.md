# Smooth Shift and Thicken — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Smooth Shift pushes polygons out along their normals and builds side walls
so the pushed polygons stay connected to the rest of the mesh, similar to a
polygon extrude, but with one shared offset per vertex instead of one per
polygon, and with a smoothing angle that decides whether neighbouring pushed
polygons stay joined or split apart along their shared edge. Thicken is the
same tool with different starting values (a preset): it additionally keeps a
reversed copy of the original polygons, so a flat sheet becomes a closed
shell.

## Inputs

- Polygon selection mode (all probes ran in polygon mode).
- Selected polygons are processed. With **nothing selected, every polygon of
  the mesh is processed** (SS10, TH12).
- Behavior in vertex or edge selection mode was not probed (Q-SS-01).

## Options

One option set serves both tools. Values in brackets are the observed values
after the tool is reset (Smooth Shift) and when the Thicken preset is
activated (Thicken).

| Option | Type | Smooth Shift | Thicken | Effect |
|---|---|---|---|---|
| Offset | distance | 0 | 0 | Distance each vertex moves along its shift direction. Negative moves the other way. |
| Scale | percent | 100 % | 100 % | Scales the shifted vertices (see Result, "Scale"). |
| Max Smoothing Angle | angle | 89.5° | 90.1° | Polygons whose normals differ by less than this share their vertices and shift as one surface; across larger angles the shared edge splits. |
| Thicken | boolean | off | on | Keep a reversed copy of the original polygons in place (closes the shell). |
| Sharp | boolean | off | off | Keep each pushed face at the full Offset from its original plane at smoothed corners (see Result). |
| Polygon Angle Weighting | boolean | off | off | Weight each polygon's normal by its corner angle when averaging vertex normals. |
| Both Sides | boolean | off | off | Split the thickness: original side moves back by half the Offset, pushed side forward by half. |

Notes:
- "Thicken" in the UI is a preset of Smooth Shift (same option set). After
  activating the preset, a tool reset returns all options to the Smooth Shift
  defaults (D0).
- Angles are entered in degrees. Scale 100 % is neutral.

## Interaction

Per the guide, the tool has two viewport handles, one for Offset and one for
Scale, and can also be applied from the properties panel. Handle behavior was
not probed (Q-SS-02). All probes applied the tool numerically.

## Result

Terms: the **shift direction** of a vertex is computed from the normals of
the processed polygons around it that belong to the same smoothing group (see
next bullet). It is the normalized average of those polygon normals, and the
vertex moves by Offset along it.

**Smoothing groups.** Two processed polygons that share an edge stay joined
when the angle between their normals is below Max Smoothing Angle; above it,
they get separate copies of the shared vertices.

- 90° corner with Max Smoothing Angle 89.5° or 0°: split (SS03, SS04, TH05).
- 90° corner with 90.1° or 180°: joined (SS05, TH06, TH13).

**Joined vertices** move by exactly Offset along the normalized average
normal, so at a corner the faces end up less than Offset from their old
planes:
- L-shape (floor + wall, 90°), shared edge vertex moves to (0.070711,
  0.070711) for Offset 0.1 (TH06).
- Cube corner with all three faces processed, moves 0.1 along the
  diagonal, to ±0.557735 (SS05, TH13).

**Sharp** (TH07a) instead puts a joined vertex where the pushed faces' offset
planes meet: L-shape shared edge goes to (0.1, 0.1). With faces split by the
smoothing angle, Sharp on gave an unexplained result (SS11b: every pushed
vertex moved 0.3 for Offset 0.1; Q-SS-03).

**Polygon Angle Weighting** (TH08a/b, cube made of 12 triangles, all
selected, Offset 0.1):
- Off: each adjacent polygon counts once. A corner with two triangles on
  two faces and one on the third moves to (-0.533333, -0.566667, -0.566667)
  from (-0.5, -0.5, -0.5).
- On: each polygon counts by its corner angle, so every corner moves along
  the true diagonal (±0.557735).
- On quads with equal corner angles (SS11c), it makes no difference.

**Topology, Thicken off** (Smooth Shift):
- The processed polygons are moved: they keep their identity and facing.
- On every boundary edge of the processed region, a quad wall joins the
  moved edge to the original edge. Boundary means: shared with an
  unprocessed polygon, or an open mesh border (SS08: a 2×1 region of an
  open grid gets 6 walls). For positive Offset the walls face outward
  (same side as the pushed surface).
- Where a shared edge splits (smoothing angle), each side gets its own copy
  of the edge, and the gap is filled with:
  - one quad between the two moved copies of the edge, and
  - one triangle at each end of the edge. The triangle joins the two moved
    copies of the end vertex and the original vertex, when that original
    vertex is still used by an unprocessed polygon (SS03: cube top + front
    → 8 new points, 9 new polygons: 3 + 3 walls, 1 bridge quad, 2
    triangles).
- Original vertices that no polygon uses after the operation are removed:
  - SS07/SS10: the whole cube processed, 90° edges split. The result has 24
    points (6 moved faces, 12 bridge quads, 8 corner triangles) and none of
    the original corners.
  - TH11: Thicken off on a full grid. The interior original vertex is gone.

**Topology, Thicken on**: as above, plus a copy of every processed polygon
left at the original position with reversed corner order (faces the other
way). Examples:
- Single quad, Offset 0.1 (TH02): 8 points, 6 polygons, a closed box.
- 2×2 grid (TH04): 18 points, 16 polygons (4 top, 4 bottom, 8 border walls).
- Closed cube, all faces, 90.1° (TH13): 16 points, 12 polygons. Outer shell
  pushed, inner shell reversed, no walls (there is no border).

**Offset 0** still builds everything, with the moved copy on top of the
original (TH01: zero-thickness box, 8 points, 6 polygons; SS01 likewise for
Smooth Shift).

**Negative Offset**:
- Smooth Shift (SS09): the moved polygon goes the other way and keeps its
  facing; the walls are built the same way, so they face into the shifted
  region.
- Thicken (TH03): the moved polygon is flipped and the walls are reversed,
  so the shell still faces outward. The copy at the original position keeps
  the original facing. The guide describes the same intent (no inside-out
  result).

**Scale** scales the moved vertices after the shift, toward a fixed centre:
- SS06 (cube top face, Offset 0, Scale 20 %): the moved face's corners go
  to (±0.1, 0.1, ±0.1).
- TH10 (grid ±1 at z 0, Offset 0.1, Scale 50 %): corners go to (±0.5, ±0.5,
  0.05).
- Both fit scaling about the world origin, which is also the mesh centre in
  both probes. Which centre is used is Q-SS-04. Original-side vertices are
  not scaled.

**Both Sides** (TH09, flat grid, Offset 0.1): the original side moves back
to z = −0.05 and the pushed side to z = +0.05. On a closed-corner input
(SS11d, Thicken off) the backward move of original vertices sometimes went
along a different face's normal (Q-SS-05). The exact positions are in the
case.

All new polygons are plain faces with the Default material in every probe.
Whether they inherit material, part or polygon type from their source
polygon was not tested (Q-SS-06).

## Selection after

Polygon mode is kept.
- The moved polygons stay selected (SS02, SS03, SS08, TH02, TH04).
- Where edges split, the bridge quads between moved copies are selected too.
  Corner triangles and side walls are not (SS03: 3 selected; TH05: 3).
- SS07 (all six cube faces selected): the 6 moved faces + the 12 bridge
  quads.
- Nothing selected before:
  - Smooth Shift (SS10): only the 12 bridge quads end up selected.
  - Thicken (TH12): nothing selected.
- The reversed copies made by Thicken are not selected.

## Refusals

None observed: every probe applied without a message. The mesh tools refuse
to activate on an empty mesh layer (D0); the message wasn't recorded
(Q-SS-07).

## Undo

Not probed (Q-SS-08).

## Acceptance cases

All in `cases.json` (compare geometry, not indices; see its
`comparison_rule`).

- SS01 defaults on cube top face (zero offset, full topology)
- SS02 cube top, Offset 0.1
- SS03/SS04 cube top + front, angle 89.5° / 0° (split edge, bridge quad, triangles)
- SS05 same at 180° (joined, diagonal corners)
- SS06 Scale 20 %
- SS07 all six faces; SS10 nothing selected (same geometry, different selection)
- SS08 two quads of an open grid
- SS09 negative Offset
- SS11a–d Thicken / Sharp / Angle Weighting / Both Sides on SS03's input
- TH01–TH03 single quad: zero, positive, negative Offset
- TH04 full grid; TH05/TH06 L-shape split / joined; TH07a/b Sharp on / off
- TH08a/b triangulated cube, weighting off / on
- TH09 Both Sides; TH10 Scale 50 %; TH11 Thicken off; TH12 nothing selected; TH13 closed cube

## Open questions

See `QUESTIONS.md`.

## Open questions and decisions

- **Q-SS-01** What happens in vertex or edge selection mode (processes
  polygons touching the selection, all polygons, or nothing)?
- **Q-SS-02** Handle behavior: which handle drives Offset and which Scale,
  drag direction and scaling per pixel, live preview, what a new click does.
- **Q-SS-03** Sharp with split smoothing groups. In SS11b (cube top + front,
  89.5°, Offset 0.1) every pushed vertex moved 0.3, not 0.1. What rule
  produces this? Needs probes at other Offsets and with one face only.
- **Q-SS-04** Scale centre. Both probes fit the world origin, which was
  also the mesh and bounding-box centre. Is it the origin, the action
  centre, the selection's centre, or the mesh centre? Probe an off-origin
  mesh.
- **Q-SS-05** Both Sides on a closed corner (SS11d). Some original vertices
  moved back along an adjacent unselected face's normal rather than the
  selected face's. Confirm and find the rule, or decide to treat it as a
  quirk not to copy.
- **Q-SS-06** Do new walls, bridge quads and Thicken copies inherit the
  material tag, part and polygon type (Subdivs, Catmull-Clark) of their
  source polygons? Are UVs and weight maps carried along?
- **Q-SS-07** Exact refusal wording and when it appears (empty layer,
  no polygons).
- **Q-SS-08** Undo: one step per apply? What does undo restore, including
  selection?
- **Q-SS-09** Max Smoothing Angle exactly equal to the corner angle (90° on
  a cube): split or joined?
- **Q-SS-10** Why does SS10 (nothing selected) leave only the bridge quads
  selected? Confirm on another mesh.
- **Q-SS-11** Allowed ranges and clamping for Offset, Scale (negative?) and
  Max Smoothing Angle.
- **Q-SS-12** Non-planar polygons, polygons with more than four sides, and
  mixed inputs: is the polygon normal the same one Polygon Extrude uses?
