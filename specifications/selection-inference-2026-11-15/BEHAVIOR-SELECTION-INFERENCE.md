# Selection inference: element tools in any selection mode

Version 1.1 (2026-11-15). Behaviour only. Status: MEASURED (hands-on
checks M01-M10 in section 2b, 2026-11-15). Items still marked **[PROBE]**
were not measured. Items marked **[OP]** are Operandi's own
choice, not reference behaviour.

## 1. What this is

Some tools work on one element type: points (Set Position, Vertex Bevel,
Join), edges (Edge Bevel, Edge Extrude, Crease) or polygons. Today Operandi
refuses most of them unless the selection mode matches ("Vertex Bevel needs
selected points in Vertex mode").

New rule: the tool runs from another selection mode by working out its
elements from the current selection. Selected edges or polygons give their
points; selected polygons give the edges around them.

This is one shared step in the tool pipe, run before every other layer
(action centre, falloff, symmetry, snapping, constraint, tool). Tools do not
convert selections themselves.

## 2. Reference behaviour (documented)

SI-R01. With nothing selected, every element of the current mode counts as
selected for tools and commands.

SI-R02. What each mode hands to tools ("implicit selection"):

| Current mode | Points the tool gets | Edges the tool gets | Polygons the tool gets |
|---|---|---|---|
| Vertex | the selected points | none listed | none listed |
| Edge | the points of the selected edges | the selected edges | none listed |
| Polygon | the points of the selected polygons | the **outline edges** of the selected polygon group | the selected polygons |

SI-R03. Documented examples: Vertex Bevel runs on a polygon selection; an
edge tool run on several selected polygons acts on "all the edges around the
polygon group" (the outline, not the inside edges); edge Split in Polygon
mode splits the outline edges of the selected polygons.

SI-R04. Nothing documented converts upwards (points to edges, points to
polygons, edges to polygons) for tools. Upward conversion exists only as the
separate Convert Selection command, which also switches mode. Measured
anyway: see M06, M07.

SI-R05. Some commands mean something different in each mode and do NOT
infer: Remove/Delete removes whatever type is selected; Collapse, Split and
Make each have a per-mode version. Edge Remove keeps the points; polygon
Remove removes polygons.

SI-R06. Nothing documented says the selection mode changes when a tool runs
on an inferred selection. Measured: M03.

## 2b. Measured (3x3 grid of 1-unit quads, points numbered row by row
from 0 to 15, polygons 0 to 8; and a unit cube)

M01. Vertex Bevel with 2 polygons selected in Polygon mode gives exactly
the same mesh as Vertex Bevel on their 6 corner points in Vertex mode.
M02. Vertex Bevel with 2 edges selected in Edge mode = the same on their 4
points in Vertex mode.
M03. Afterwards the mode is unchanged. The polygons you selected stay
selected. The tool's own result (the cut points) is the point selection, so
it shows when you switch to Vertex mode. Same in Edge mode.
M04. Nothing selected in Polygon mode: Vertex Bevel bevels every point
(same as Vertex mode with nothing selected).
M05. Set Position in Edge mode with nothing selected moves every point.
With one edge selected it moves its 2 points.
M06. **Upward to edges works:** Edge Bevel in Vertex mode with 2 adjacent
points selected = Edge Bevel on the edge between them.
M07. **Upward to polygons does nothing:** Polygon Bevel in Vertex mode (4
points of a quad) or Edge mode (its 4 edges) runs without error and changes
nothing.
M08. Edge Bevel and Edge Extrude from a 2x2 block of selected polygons
bevel/extrude the block's outline edges, but the inside edges are not
ignored cleanly: where inside edges meet, a small corner cut appears (a
diamond at the block's centre point), and open-border corners distort. The
result matches neither "outline only" nor "every edge".
M09. Edge Bevel with every polygon of a cube selected (no outline) cuts the
8 corners only (24 points, 14 polygons), like a Vertex Bevel. It does NOT
bevel the 12 edges. With 5 of 6 faces selected, the hole's 4 edges bevel
plus corner cuts.
M10. Join from 2 selected edges in Edge mode works. Join Averaged = the
same as on the 4 points. Plain Join lands on a different point than in
Vertex mode (the first point of the last edge, not the last point
selected), so its target is effectively arbitrary.

## 3. The shared rule

SI-01. **Points and edges only.** A point tool takes points from an edge or
polygon selection. An edge tool takes edges from a polygon selection, or
from a point selection (M06). Nothing ever becomes polygons: polygon tools
keep refusing in Vertex and Edge mode (M07 shows the reference silently does
nothing; Operandi says why instead).

SI-02. **Conversions:**
- Edges to points: both end points of every selected edge.
- Polygons to points: every corner point of every selected polygon.
- Points to edges: every edge whose two end points are both selected (M06).
- Polygons to edges: the **outline** of the selected polygons: an edge
  counts when exactly one of its polygons is selected, or when it is an open
  border edge of a selected polygon. Inside edges (both polygons selected)
  are left out. This is the same set as Operandi's existing Boundary
  selection, NOT "Convert to Edges" (which takes every edge). This is a
  clean version of M08: the reference also leaves corner cuts where inside
  edges meet; Operandi does not **[OP]**.
- Polygons with no outline (a whole closed mesh selected, M09): every edge
  of the selected polygons **[OP]**. The reference cuts the corners
  instead, which nobody asks for when they pick Edge Bevel.

SI-03. **Polygons are never inferred.** A polygon tool in Vertex or Edge mode
refuses with a message, for example "Polygon Bevel needs polygons: switch to
Polygon mode, or Convert the selection" **[OP]** (the reference silently
does nothing, M07).

SI-04. **Only the current mode's selection is read.** Selections kept in
other modes are ignored, even if they exist. (A tool that natively reads two
selections, such as Vertex Tear, keeps doing so; see section 6.)

SI-05. **The mode does not change.** The tool runs, and you stay in the mode
you were in. (M03)

SI-06. **The selection you made is kept** as it was (where its elements
still exist after the edit). The tool's own result selection (for example
Vertex Bevel's cut points) goes into that tool's element type, so it is
there when you switch to that mode. (M03)

SI-07. **Order.** Some tools care about selection order. Inferred points are
ordered by walking the selection in the order it was made: for an edge, its
first then second point; for a polygon, its corners in winding order from
its first corner. A point already listed is not added again. Tools whose
result depends on order (section 4, "order") refuse inference instead **[OP]**.

SI-08. **Say what happened.** While the tool is active, the status line
says what it is working on, for example "Vertex Bevel on 12 points from 3
polygons". Never silent (same principle as the symmetry status rule).

SI-09. **One undo step**, as if the elements had been selected natively.
Undo restores the selection you had.

SI-10. **Per mesh.** Each active mesh converts its own selection. Hidden
elements are left out, as in native mode. Locked elements follow the
tool's native rule.

SI-11. **Live tools re-read nothing.** The inferred set is worked out once,
when the tool starts acting, and stays fixed while you drag. (A re-drag on
the same bevel uses the same set.)

## 4. Which tools

Columns: "From" = the modes it now also runs in. "Native" = unchanged.

### Point tools (take points)

| Tool / command | Name | From | Notes |
|---|---|---|---|
| Set Position | vert.set | Edge, Polygon | Already infers. **Fix:** Edge mode with nothing selected = every point (M05); today Operandi refuses. |
| Vertex Bevel | vert.bevel | Edge, Polygon | Documented (SI-R03). |
| Vertex Extrude | vert.extrude | Edge, Polygon | |
| Vertex Merge (tool) | vert.merge | Edge, Polygon | Distance merge on the inferred points only. |
| Vertex Slide | vert.slide | Edge, Polygon | Different from Edge Slide; both exist. |
| Weld | vert.weld | Edge, Polygon | Uses the inferred points as the set to weld from. |
| Join / Join Averaged | vert.join | Edge, Polygon | Both run (M10). Plain Join lands on the last point in SI-07 order (the second point of the last selected edge; the last corner of the last selected polygon) **[OP]**: deterministic where the reference is arbitrary. |
| Align (points) | vert.align | Edge, Polygon | |
| Center | vert.center | Edge, Polygon | |
| Split (points) | vert.split | No | Mode-aware: edge Split and polygon Split are different commands (SI-R05). |
| Collapse, Remove | vert.collapse, vert.remove | No | Mode-aware (SI-R05). |
| New Vertex, Create | vert.new | No | Makes points, takes none. |
| Reorder, Sync Order | vert.reorder, vert.syncOrder | No | Order based; Sync Order already reads polygons natively. |
| Tear | vert.tear | No | Native mixed selection (section 6). |
| Split Edge, Remove Edge | vert.splitEdge, vert.removeEdge | No | These are edge commands under another name. |

### Edge tools (take edges)

| Tool / command | Name | From | Notes |
|---|---|---|---|
| Edge Bevel / Chamfer | edge.bevel, edge.chamfer | Vertex, Polygon | Outline from polygons (SI-02), edges between points (M06). |
| Edge Extrude | edge.extrude | Vertex, Polygon | Outline. |
| Edge Extend | edge.extend | Vertex, Polygon | Outline. On a closed object the outline is not a border; the tool's native rule decides. |
| Edge Slide | edge.slide | Vertex, Polygon | Outline. |
| Edge Crease | edge.crease | Vertex, Polygon | Outline. SI-Q03. |
| Edge Relax | edge.relax | Vertex, Polygon | Outline. |
| Split Normals | edge.splitNormals | Vertex, Polygon | Outline: gives a hard rim around the polygon group. |
| Split (edges) | edge.split | Vertex, Polygon | Documented: splits the outline (SI-R03). |
| Remove, Collapse, Subdivide, Spin | edge.remove, edge.collapse, edge.subdivide, edge.spinQuads | No | Mode-aware; polygon versions exist. |
| Join | edge.join | No | Needs exactly two edges in order. |
| Bridge, Fill Quads, Grow Quads | edge.bridge, edge.fillQuads, edge.growQuads | No | Need a hole or border outline; a polygon outline is the wrong input. |
| Add Loop, Add Point, Knife, Unbevel | edge.addLoop, edge.addPoint, edge.knife, edge.unbevel | No | Click tools or exact-edge tools. |

### Polygon tools

No change (SI-01, SI-03). Polygon tools keep refusing in Vertex and Edge
mode, with the message in SI-03.

### Tools that already run in every mode

Transforms, Tweak, Deform, slices, Duplicate tab, Poly Haul: unchanged.
Their "what moves" rule (points of the selection) already follows SI-02.

## 5. Edge cases

SI-12. **Empty selection.** Nothing selected in the current mode means the
tool behaves exactly as it does in its own mode with nothing selected: Vertex
Bevel with nothing selected in Polygon mode = Vertex Bevel with nothing
selected in Vertex mode. A tool that refuses an empty selection natively
(for example the Merge command's explicit-selection rule) refuses here too.
Follows SI-R01; measured M04, M05.

SI-13. **Mixed selections.** Operandi stores one selection per mode; only
the current one is read (SI-04). A selection spread over several meshes
converts per mesh (SI-10).

SI-14. **Shared points.** Points inferred from selected polygons are also
used by unselected neighbours. The tool acts on the point, so neighbours
change too (a Vertex Bevel on a selected polygon's corners cuts into the
polygons around it). This is expected, same as selecting those points by
hand. The status line (SI-08) is the warning.

SI-15. **Outline of everything.** With every polygon of a closed mesh
selected there is no outline, so the edge tool takes every edge of the
selected polygons (SI-02). The status line says "all edges of 6 polygons
(no outline)". **[OP]**; the reference cuts corners instead (M09).

SI-16. **Symmetry.** Inference runs first, symmetry second. With symmetry on,
the native selection already includes mirrored partners (Operandi's
symmetric select), so the inferred set already has both sides. The symmetry
layer then treats the inferred set exactly as a native one (centre points
pair with themselves). A polygon that straddles the centre gives centre
points; nothing special. If the selection is one-sided (made with symmetry
off), the symmetry layer mirrors the edit as it does for a native one-sided
selection.

SI-17. **SubD surface picking.** Inference is topological: it works on the
cage element ids, so SubD makes no difference to which elements are chosen.
The display follows the SubD surface rule: while the tool is active the
inferred elements are drawn (highlighted lightly) on the smooth surface,
using the shared surface point/edge path, never cage positions. Handles that
sit on elements sit on the surface. Snapping still targets the cage.
(SI-Q05)

SI-18. **Action centre "Selection"** uses the inferred elements (what the
tool acts on), so a Vertex Bevel from polygons centres on the points'
average. **[OP]**

SI-19. **Falloff** weights are per point and don't care where the points
came from.

SI-20. **Item mode** is not a source. Tools that already run in Item mode
(Set Position: every point of the mesh) keep doing that; others keep
refusing.

SI-21. **Keyboard and toolbar.** A mode-aware key or button (one key that
picks Bevel, Edge Bevel or Vertex Bevel by mode) keeps picking by mode.
Inference only matters when a point or edge tool is chosen directly (its
own button, command line, or a search).

## 6. Tools that read more than one selection

Vertex Tear reads the selected polygons and the selected points together.
It keeps its own rule and is not touched by inference.

## 7. Tool card

The tool card's start-mode list gains the inferred modes for the tools in
section 4 marked "From", and a field saying which element type the tool
takes (points or edges). The shared step reads that field. A test should
check: every card with a "takes" field runs from each listed mode, and every
"No" tool still refuses with its current message.

## 8. Open questions (each has a default the builder uses)

- SI-Q01. Upward conversion. ANSWERED by measurement: points to edges yes
  (M06), anything to polygons no (M07). Operandi refuses with a message.
- SI-Q02. Mode and selection afterwards. ANSWERED (M03).
- SI-Q03. Polygons to edges for Crease and Split Normals: outline or every
  edge? Default: outline, as for every edge tool. Not measured.
- SI-Q04. Plain Join from edges/polygons. Reference runs it with an
  arbitrary target (M10). Default: runs, lands on the last point in SI-07
  order [OP].
- SI-Q05. Show the inferred elements while the tool is active? Default:
  yes, a light highlight on the SubD surface [OP].
- SI-Q06. Edge tools in Vertex mode. ANSWERED: yes, edges between selected
  points (M06).
- SI-Q07. Polygons to edges: reference leaves corner cuts where inside edges
  meet (M08) and cuts corners on a whole closed selection (M09). Default:
  clean outline, and every edge when there is no outline [OP]. APPROVED
  by Jason 2026-11-15 as a deliberate difference.

## 9. Not measured

Edge Crease, Edge Slide, Edge Relax and Split Normals from polygons; Vertex
Slide, Weld, Align and Center from edges/polygons. They follow the same
rule; nothing measured suggests otherwise.
