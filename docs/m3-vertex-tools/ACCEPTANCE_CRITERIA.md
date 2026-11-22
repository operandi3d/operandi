# Milestone 3 Vertex panel — basic acceptance criteria (2026-10-30)

Written before implementation. Source order: Jason's goal for this session
("implement the tools in milestone 3 (vertex tools); ensure they all comport
with the command convention"), the public Modo 17.1 user guide's vertex pages
(guide precedence, 2026-10-30), `M3_VERTEX_BASE.md` draft minimums, and
`command-conventions-2026-10-27` (removed 2026-11-05) (CC-001..012). Where the guide is silent the
rule below is **provisional** and is listed in the open-items section of the
acceptance packet.

Common to every row (shared gate):

- Context: one visible active mesh, Vertex mode (New Vertex and Create also
  work with no selection; Make Polygon and Show Vertices have their own
  context). Invalid or empty selections are refused atomically with a status
  message; nothing changes and no history entry is added.
- One undo step per edit; undo/redo restore the complete scene. Live tools:
  Drop commits once, Cancel/Esc restores the pre-tool scene.
- Untouched points, polygons, material/part tags and sparse point/corner data
  are preserved. New corners without a source corner are refused while a map
  holds values there (same policy as the M2 Edge commands).
- Native save/reopen keeps the result.
- Command route: every row is callable from command entry (`tool.set <id>
  on|off|?` for tools, a verb for one-shots), listed by `help` and `help
  <verb>`, and malformed arguments refuse before mutation (CC-004/008).

## Interactive tools

| ID | Acceptance (basic) |
|---|---|
| MV-T01 Create | `tool.set vert.create on`; each viewport click adds a point on the work plane (preview); X/Y/Z fields edit the last point; Drop commits all points as one edit and selects them in order; Cancel discards. No selection required. |
| MV-T02 Vertex Bevel | Selected manifold vertex with ≥3 polygons: each incident polygon's corner is replaced by two points at Inset along its two edges; one new polygon fills the hole (one point per incident edge, wound with the surface). Inset 0 = no change; Inset clamped below each edge length. Round Level is deferred (field shown, only 0 accepted). |
| MV-T03 Vertex Extrude | Selected interior vertex: one new point per incident edge at Width along it; the vertex moves Extrude along its normal; each incident polygon's corner is replaced by the new edge points; a ring of triangles joins the moved vertex to them. Width > 0 required when Extrude ≠ 0. |
| MV-T04 Merge Tool | Distance handle/field; preview merges selected points within Distance (single-linkage groups) into the most recently selected member's position; polygons heal; 1/2-point polygons removed unless Keep 1-Vertex Polygons. Drop commits, Cancel restores. |
| MV-T05 Weld | Click mode (`tool.set vert.weld on`): with points selected, clicking a target point welds the selected points onto it (one undo step); with none selected, the first click picks the source, the second the target. Command `vert.weld target:<index>`. *Provisional* (no guide page). |
| MV-T06 Tear | Live tool with X/Y/Z handles: needs selected polygons and a selected point on them that an unselected polygon also uses; the selected polygons' corners at that point get one new point, which follows the drag; the other polygons stay. Zero drag = no change. Behaviour set by Jason (2026-10-31); no guide page. |
| MV-T07 Slide | `S` in Vertex mode; one Slide handle; each selected point moves along its incident edge best aligned with the slide direction, as a fraction (−1..1) of that edge's length; topology unchanged; stops at the far end. |

## Command and display rows

| ID | Acceptance (basic) |
|---|---|
| MV-C01 New Vertex | Dialog X/Y/Z (or `vert.new <x> <y> <z>`) adds one point at that world position to the active mesh; it becomes the only selected point. |
| MV-C02 Make Polygon | `P` / `poly.make` in Vertex mode: ≥3 ordered selected points (distinct, nondegenerate) make one polygon in selection order (right-hand normal); material/part from a neighbouring polygon or defaults. Edge-mode Make unchanged. |
| MV-C03 Show Vertices | Toggles drawing of all point dots in Vertex mode; no scene, selection or history change. `view.showVertices <on|off|?>`. |
| MV-C04 Split | `vert.split`: each selected point used by n ≥ 2 polygons becomes n coincident points, one per polygon; moving a polygon reveals the gap. |
| MV-C05 Split Edge | `vert.splitEdge`: every edge whose two endpoints are selected gets a point at its middle in all polygons using it. *Provisional reading.* |
| MV-C06 Split Polygon | Existing `poly.split` (Ctrl+L) on two selected points of one polygon; Vertex panel row routes to it. |
| MV-C07 Set Position | Existing `vert.set` dialog extended to Vertex mode: selected points get the axis value; other coordinates untouched. |
| MV-C08 Align | `vert.align [axis:all|x|y|z|xy|yz|zx] [uniform:true|false]`: interior selected points move onto the line from the first to the last selected point (projection, or even spacing with Uniform), restricted to the chosen axes. |
| MV-C09 Merge | Dialog / `vert.merge [range:auto|fixed] [dist:<m>] [keep:true|false]`: Automatic merges exactly coincident selected points; Fixed merges within Distance; result at the most recently selected member; heal as Merge Tool. |
| MV-C10 Join | `vert.join`: all selected points weld into the last selected and move to its position; polygons heal. |
| MV-C11 Join Averaged | `vert.join average:true`: as Join, at the average position. |
| MV-C12 Collapse | `vert.collapse` (Ctrl+Backspace): each selected point is removed and its incident polygons merge into one polygon (no hole); boundary points are supported. |
| MV-C13 Remove | `vert.remove` (Shift+Backspace): selected points and every polygon using them are deleted. |
| MV-C14 Remove Edge | `vert.removeEdge`: edges whose two endpoints are selected are dissolved (their two polygons merge), as Edge Remove. |
| MV-C15 Reorder Index | `vert.reorder mode:swap|top|bottom`: point indices permuted per the guide's three modes; faces, selection and point data follow; shape unchanged. |
| MV-C16 Sync Index Order | `vert.syncOrder`: run once on two selected polygons of the source mesh (captured), then on the same two polygons of the target mesh; the target's points are renumbered to match the source; shape unchanged. *Two-step capture is provisional* (Operandi selection is per item). |

## Unsupported in this pass (refused, not ignored)

Round Level > 0, Use Material, curves/Bezier inputs, Test Morph Map
Positions, Merge Discontinuous UVs option, Align planar modes beyond the
axis mask, Sync on mismatched topology, Weld/Tear on polylines, falloff /
symmetry / action-center composition for the new tools.
