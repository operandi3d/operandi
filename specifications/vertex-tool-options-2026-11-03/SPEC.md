# Vertex tool options — behavior spec v2 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

**Status: v2 (partial).** Every Slide default recorded in v1 was confirmed;
one was corrected (Mode and Vertex mode reset differently in Edge mode).
Vertex Bevel, Vertex Extrude and Merge Tool defaults were added.

This spec lists every option behind the Vertex panel's seven tools and its
command dialogs: name, type, default, range, what it does, and how sure we
are. Option IDs are VTO-T01… (tools) and VTO-C01… (commands).

## Evidence marks

Every option carries one of these in its **Source** column.

| Mark | Meaning |
|---|---|
| **M** | Measured: a numeric case with exact before/after geometry exists in another spec (cited by ID). Build to the oracle. |
| **R** | Recorded: the default value was read back from Modo's own tool settings by a probe. The value is exact; the behavior is not measured. |
| **G** | Guide only: from the Modo 17.1 user guide, restated in our words. Treat behavior as intent, not as an oracle. |
| **O** | Open: neither the guide nor a probe says. Don't invent it; see QUESTIONS.md. |

"Default: not recorded" means no probe read it back. Where a guide sentence
states a default ("off by default"), the mark is **G**.

## Measured evidence this spec leans on (already in the repo)

- `mesh-commands-2026-11-02/BEHAVIOR-VERTEX.md`: CMD-V-01…06 (Merge, Join,
  Collapse, Remove, Split, Align).
- `mesh-commands-2026-11-02/BEHAVIOR-BATCH-3.md`: CMD-B3-02 (Collapse rule).
- `new-point-uvs-2026-11-02/BEHAVIOR-PER-TOOL.md`: UVN-T-04 (Vertex Bevel),
  UVN-T-05 (Vertex Extrude).
- `falloff-2026-11-01/BEHAVIOR-TOOLS.md` FAL-T-03 and
  `falloff-2026-11-01/BEHAVIOR-BATCH-3.md` FAL-B3-02.
- `add-loop-edge-slide-2026-11-02/`: Edge Slide; Vertex Slide is ES-Q03,
  still open.
- `modeling-popup-menu-2026-11-01/`: Split Vertex on a cube from the popup.

## Not covered

Procedural (Mesh Ops) versions and their extra options; snapping; symmetry;
Work Plane behavior beyond naming the option; undo. Shared gaps across all
tools (falloff, profiles, tool states) are only mentioned where a vertex
measurement exists.

# Part 1: Vertex panel tools


## VTO-T01 Create

Interactive point placement. The guide has no page for the interactive
tool (its Create Vertex page describes the X/Y/Z dialog, see VTO-C01).

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Click to place | gesture | n/a | n/a | Each click on the work plane adds one point there. | G |
| Position X / Y / Z | distance ×3 | the last click | any | Shows and edits the most recently placed point. | G |
| Drag latest point | gesture | n/a | n/a | Dragging the newest point (or Shift+drag anywhere) repositions it. | O |
| Point Polygon | boolean | not recorded | on/off | Also makes a one-point polygon for each new point, so the points survive cleanup that deletes floating points. | O |

---

## VTO-T02 Vertex Bevel

Replaces each selected point with a small cap polygon. One handle only.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Inset | distance | **0** | not recorded (≥ 0 assumed) | How far along each incident edge the new points sit from the original point. | R, M (UVN-T-04, FAL-T-03) |
| Use Material | boolean | **off** | on/off | Tag the new cap polygon with the material chosen below. | R, G |
| Material Name | text | **empty** | material names | Material for the cap; only active when Use Material is on. | R, G |
| Round Level | integer | **0** | not recorded (≥ 0 assumed) | Adds extra points along the new edges to round the cap. | R, G |

**Measured behavior (UVN-T-04).** Inset 0.2 on an interior point of a 3×3
grid where four quads meet: the point is removed, one new point appears on
each of its four edges at 0.2 from it, and they form one new cap quad.
16 → 19 points, 9 → 10 polygons. Afterwards the cap's points and edges are
selected. The cap takes UVs from the polygons it was cut out of (see
UVN-T-04 for the seam case).

**Falloff (FAL-T-03).** Inset is multiplied by the falloff weight at the
point: weight 0.5 with Inset 0.2 gives new points 0.1 away.

**Input.** The guide also allows curves (not Bezier). Border points,
valence 3 and above 4, and Round Level above 0 are unmeasured (QUESTIONS
VTO-Q02).

---

## VTO-T03 Vertex Extrude

Pulls each selected point out along its normal and builds a ring of new
points around it, giving a little pyramid.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Extrude | distance | **0** | not recorded (negative not tested) | How far the selected point moves out along its normal. | R, M (UVN-T-05) |
| Width | distance | **0** | not recorded | How far along each incident edge the new ring points sit from the original position. | R, M (UVN-T-05) |
| Use Material | boolean | **off** | on/off | Tag the new polygons with the material below. Off: new polygons get no material tag. | R |
| Material Name | text | **empty** | material names | Material for the new polygons. | R |

**Measured behavior (UVN-T-05).** Width 0.2, Extrude 0.2 on the interior
valence-4 grid point: one ring point per incident edge at 0.2 from the
original, the original point raised 0.2 along its normal as the apex, and
four triangles from the ring to the apex. 16 → 20 points, 9 → 13 polygons.
Afterwards only the apex is selected.

**Falloff (FAL-B3-02).** Both Extrude and Width are multiplied by the
selected point's weight. A point at weight 0 is left unextruded.

**Note.** Modo's default Width is 0. Operandi currently refuses Width 0.
What Modo builds at Width 0 is unmeasured (VTO-Q03).

---

## VTO-T04 Merge Tool

Interactive version of Merge: set a distance by dragging in the viewport or
with a mini-slider, and points that close together merge.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Distance | distance | **0.001** (after Reset; before Reset it shows the last drag) | ≥ 0 | Points this far apart or closer merge. Drag in the viewport or use the mini-slider to set it. | R, G |
| Keep 1-Vertex Polygons | boolean | **off** | on/off | When on, polygons squashed to one or two points are left in place instead of deleted. | R, G |
| Test Morph Map Positions | boolean | **off** | on/off | When on, points merge only if they also coincide in every morph map. | R, G |

Two more settings exist with no guide label: one reads **on** (by its name, it allows points of the same polygon to merge) and one reads **off** (by its name, merges each point to its closest neighbour). Don't build them until they're described.

The guide says "less than or equal to". Where the merged point lands is not
stated for the tool. The Merge command was measured landing at the midpoint
(CMD-V-01). See VTO-Q04.

---

## VTO-T05 Weld

No guide page. The Weld tool has no settings of its own (read back 2026-11-05).

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Source then target | gesture | n/a | n/a | Pick a point, then the point to weld it to; the first point joins the second. Operandi's own reading; the order is not confirmed. | O |
| Other options | ? | ? | ? | Unknown. | O |

A hands-on check is planned (VTO-Q05).

---

## VTO-T06 Tear

No guide page for the tool. Poly Haul's Tear mode tears one polygon's
corner away from its neighbours when Select Through is off; the Vertex
panel Tear is assumed to work the same way on the picked corner.
Operandi's current reading was decided on 2026-10-31.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Tear and drag | gesture | n/a | n/a | Gives the picked corner its own point and drags it away from the other polygons. | O |
| Other options | ? | ? | ? | Unknown. | O |

A hands-on check is planned (VTO-Q06).

---

## VTO-T07 Slide

One Slide tool serves points and edges. In Vertex mode it shows a Linear
handle and a Face handle by default, and the guide says Select Through must
be on. All edge behavior is in `add-loop-edge-slide-2026-11-02/`. Nothing
has been measured on points yet (ES-Q03).

Defaults below were read back from Modo 17.1v1 on 2026-11-05: tool on,
Reset, every setting read (one read-only probe, the probe cases),
once with a point selected in Vertex mode and once with an edge selected in
Edge mode. Every value was the same in both except the two marked **mode**:
**Reset depends on the selection mode.**

| Option | Type | Default after Reset | Range | What it does | Source |
|---|---|---|---|---|---|
| Mode | choice | **Linear** in Vertex mode, **Radial** in Edge mode | Linear, Radial | Linear moves along a direction; Radial widens or narrows a loop (edges). | R (mode), G |
| Vertex mode | choice | **Edge** in Vertex mode, **None** in Edge mode | None, Edge, Face, Normal | How points slide: along connected edges, across the surface (Face), along the normal, or not at all. Only Edge is described by the guide; the others are names only. | R (mode) |
| Face handle | boolean | **on** | on/off | Shows the Face handle for points, which slides across the surface rather than along one edge. | R, G |
| Direction X / Y / Z | distance ×3 | **0, 0, 0** | any | Direction for the Linear slide. Zero means "chosen from the handle". | R, G |
| Interpolation | choice | **Percent** | Distance, Percent | Whether the amount is a share of the edge or a length. | R, G |
| Percent | percent | **0 %** | −100 % to 100 % (guide) | 0 is the start position; ±100 % reaches the neighbouring point or edge. | R, G |
| Distance | distance | **0** | any | Slide amount when Interpolation is Distance. | R, G |
| Merge Vertices | boolean | **on** | on/off | Points that land on another point merge. | R, G |
| Stop at Edges | boolean | **off** | on/off | Stops the slide at the surrounding edges. | R, G |
| Duplicate | boolean | **off** | on/off | Leaves the original in place and slides a new copy. Needed for profiles. | R, G |
| Loop Slide | boolean | **on** | on/off | Edges only: keeps a loop's movement within its polygons. No effect on points. | R, G |
| Segments | integer | **0** | ≥ 0 | Profile segments; 0 lets the tool choose. | R, G |
| Preserve Curvature | boolean | **off** | on/off | Moves along a curve through the edges instead of straight. | R, G |
| Tension | percent | **100 %** | not recorded (negative allowed per guide) | How tightly Preserve Curvature follows the curve; 0 % equals off. | R, G |
| Slip UVs | boolean | **off** | on/off | Keeps existing UVs from stretching. | R, G |
| Handle Line Size | number | **2.5** | not recorded | Line width of the point handles. Display only. | R, G |
| UV Map | choice | **(none)** | maps in the mesh | Slide in a UV map instead of 3D. | R, G |
| Profile Inset | distance | **0** | not recorded | Profile width (profiles). | R, G |
| Reverse X | boolean | **on** | on/off | Profile flip; probably the guide's Reverse Direction. Mapping not confirmed. | R, G |
| Reverse Y | boolean | **off** | on/off | Profile flip; probably the guide's Reverse Inset. Mapping not confirmed. | R, G |
| Keep Aspect | boolean | **on** | on/off | Sets the profile inset from the profile's aspect ratio. | R, G |
| Snap | boolean | **off** | on/off | Not described in the guide. | R |

Six more settings exist with no guide label: three blend amounts (all 0),
a relax amount (0 %), a keep-direction switch (off) and a project switch
(off). Don't build them until they're described (VTO-Q07). The
keep-direction switch read **on** before Reset and **off** after it, so it
carries over between uses; the other values carried over from the last use
too (Mode and Vertex mode showed the previous use's values until Reset). One
more internal value (a handle index, −1) is not a user option.

---

# Part 2: Vertex commands

Marks: **M** measured, **R** default recorded from Modo, **G** guide only,
**O** open. Dialog defaults were never read back by a probe, so "not
recorded" is the norm here. Where a measured case exists, build to it.

G3 below is the 3×3 quad grid from `mesh-commands-2026-11-02/README.md`.

---

## VTO-C01 New Vertex

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| X, Y, Z | distance ×3 | not recorded | any | Makes one point at that position. | G |

## VTO-C02 Make Polygon

Builds one polygon from the selected points in selection order (the order
sets which way it faces), or from a selected edge loop (order doesn't
matter).

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Type | choice | not recorded | Faces, Subdivs, Curves, SplinePatch, Catmull-Clark, Polyline, Auto | Faces: plain polygon. Subdivs and Catmull-Clark: subdivision polygons of that kind. Curves: a spline through the points. SplinePatch: patch curve, needs 3 or 4 points. Polyline: a chain of straight segments. Auto: copies the type of a neighbouring polygon on a selected point; plain Faces when the points are floating. | G |
| Close Curve | boolean | not recorded | on/off | Curves only: joins the last point back to the first. | G |

## VTO-C03 Show Vertices

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Toggle | boolean | off (project decision) | on/off | Draws point dots. Per-viewport or global, and whether it draws in Edge/Polygon modes, are open. | O |

## VTO-C04 Split

No options. Each polygon sharing a selected point gets its own copy of the
point, all at the same place; nothing moves. **M**: CMD-V-05 (G3 point 5:
16 → 19 points). Popup case on a cube: one corner split into three
coincident points, 10 points, 6 quads, nothing selected afterwards
(`modeling-popup-menu-2026-11-01`). Curves: **G**, unmeasured.

## VTO-C05 Split Edge

What it does is **O**. Operandi currently adds a middle point to the edge
between two selected points (its own reading). A hands-on check is planned.

## VTO-C06 Split Polygon

No options known. Cuts a polygon with a new edge between two selected
points on it. **G**. Successive cuts across several polygons: **O**.

## VTO-C07 Set Position

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Axis | choice | not recorded (dialog) | X, Y, Z | Which coordinate to set. | G, M (set-position-2026-11-05) |
| Value | distance | not recorded (dialog) | any | The coordinate all selected points get on that axis. Other coordinates stay. | G, M (set-position-2026-11-05) |
| World Space | boolean | not recorded (dialog) | on/off | Value is in world coordinates instead of the item's own. | G, M (set-position-2026-11-05) |
| Work Plane | boolean | not recorded (dialog) | on/off | Value is measured on the work plane. | G, M (set-position-2026-11-05) |

## VTO-C08 Align

Lines the selected points up on the straight line from the first selected
point to the last.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Axis | choice | not recorded | All, X, Y, Z, XY, YZ, ZX | Which coordinates may change. All: points go onto the line. One axis or a plane: only those coordinates change. | M (CMD-V-06: All and Y) |
| Uniform | boolean | not recorded | on/off | Also spaces the points evenly along the line. | G |

CMD-V-06: with Uniform off, each point keeps its place along the line's
main direction and is moved onto the line (not evenly spaced).

## VTO-C09 Merge

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Range | choice | **Automatic** | Automatic, Fixed | Automatic: only points in exactly the same place merge. Fixed: use Distance. | G, M (CMD-V-01) |
| Distance | distance | not recorded | ≥ 0 | Fixed only: points within this distance merge. | M (CMD-V-01) |
| Keep 1-Vertex Polygons | boolean | **off** | on/off | When on, polygons squashed to one point are kept; off deletes them. | G |
| Merge Discontinuous UV Values | boolean | **on** | on/off | Also joins UV values that were split at the merged points, so the UV map becomes continuous there. | G |

CMD-V-01: Fixed 0.01 merges two point pairs 0.004 apart into their
midpoints (8 → 6 points). Automatic leaves them alone. The guide says the
merged point lands on the most recently selected point; the measured case
put it at the midpoint. Follow the measurement and see VTO-Q04.

## VTO-C10 Join and VTO-C11 Join Averaged

Join Averaged is Join with Average on.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Average | boolean | off for Join, on for Join Averaged | on/off | On: the joined point goes to the centroid of the selection. Off: it goes to the last selected point. | M (CMD-V-02) |
| Keep 1-Vertex Polygons | boolean | not recorded | on/off | When on, polygons squashed to one point are kept; off deletes them. | G |

CMD-V-02: G3 points 5, 6, 9: Average on → (1.333333, 1.333333); off → (1, 2).
Either way 16 → 14 points and 9 → 8 polygons. The guide says Join Averaged
shows a message with how many points were joined; not observed (hands-on).

## VTO-C12 Collapse

No options. **M**: CMD-V-03 and CMD-B3-02. In the measured quad
neighbourhoods the selected point and its edge neighbours become one point
at their average, and the polygons around the point disappear. Arbitrary
neighbourhoods and several selected points are open (CMD-Q07).

## VTO-C13 Remove

No options. **M**: CMD-V-04. The point is taken out of its polygons, which
lose a corner (quads become triangles). Polygons are not deleted.

## VTO-C14 Remove Edge

No options described. Dissolves the edges between the selected points,
merging the polygons on each side. **G/O**: only the general Remove page
covers it; border and multi-edge cases unmeasured.

## VTO-C15 Reorder Index

Changes the point numbering only; the mesh looks the same.

| Option | Type | Default | Range | What it does | Source |
|---|---|---|---|---|---|
| Mode | choice | not recorded | Swap, Top, Bottom | Swap: the selected points trade their existing numbers, handed out low to high in selection order. Top: the selected points become 0, 1, 2… in selection order and every other point follows. Bottom: every other point comes first and the selected points take the last numbers, the last selected getting the highest. | G |

## VTO-C16 Sync Index Order

No options. Select the same two polygons on a source mesh and on a target
mesh with matching topology; the target's point numbering is changed to
match. A dialog reports how many points were renumbered. **G**.

## Open questions and decisions

- **VTO-Q01 — Defaults not recorded.** Slide, Vertex Bevel, Vertex
  Extrude and Merge Tool defaults are now recorded (2026-11-05). Weld has
  no settings of its own as a tool; no separate Tear or interactive Create
  tool was found among Modo's tool names, so their options (and the command
  dialogs' defaults, which only show in a dialog) stay unrecorded.
- **VTO-Q02 — Vertex Bevel beyond one interior valence-4 point.** Border
  points, valence 3 and 5+, Round Level above 0, curves, and two selected
  points sharing an edge.
- **VTO-Q03 — Vertex Extrude at Width 0** (Modo's default) and with
  negative Extrude. Border points.
- **VTO-Q04 — Where merged points land.** The guide says on the most
  recently selected point; CMD-V-01 measured the midpoint. Check Merge and
  the Merge Tool with three points in a known selection order. Also check
  "≤" versus "<" at exactly the threshold.
- **VTO-Q05 — Weld** (hands-on): every control, click order,
  several sources.
- **VTO-Q06 — Tear** (hands-on): every control, the gesture, Select Through on
  versus off.
- **VTO-Q07 — Vertex Slide** (ES-Q03, hands-on): Linear and Face handle drags
  on a grid point; what the Face and Normal Vertex mode values do; the six
  unlabeled settings.
- **VTO-Q08 — Create:** whether Point Polygon exists in 17.1 and its
  default; the drag and Shift+drag gestures.
- **VTO-Q09 — Split Edge** (hands-on) and **Show Vertices** scope (hands-on).
- **VTO-Q10 — Uniform Align, Set Position World Space / Work Plane, Make
  Polygon types other than Faces**: unmeasured.
