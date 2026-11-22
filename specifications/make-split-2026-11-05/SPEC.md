# Make Polygon and Split Polygon — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Make Polygon

Command with one option, **Type**: Auto, Face, Subdivision, Catmull-Clark,
Spline Patch, Curve, Polyline; and **Close Curve** (curves). The **P** key
runs it with Type Auto. The menu entry asks for the type first (a dialog);
there is no default type without asking.

### From selected points (vertex mode)

1. **Order is selection order.** The new polygon visits the points in the
   order they were selected, so the order sets the facing:
   counter-clockwise from +Z faces +Z (MP01), the reverse faces −Z (MP02).
   A crossing order makes a self-crossing (bow-tie) polygon as asked
   (MP03). The corner list starts at the last-selected point; compare up
   to rotation.
2. **Point count decides the result for Auto:** 3+ points → a polygon
   (MP04 triangle, MP21 pentagon); 2 points → a two-point line polygon
   (MP05); 1 point → a one-point polygon (MP06).
3. **Facing follows connected neighbours.** When the selected points
   border existing polygons (filling the open top of a cube), the new
   polygon is wound to agree with its neighbours (outward), whatever the
   selection order and type: 7,6,5,4 and 4,5,6,7 both give the outward top
   (MP15, MP16, MP20, MP22, MP23). Floating points keep the selection
   order (rule 1).
4. **Type Auto** takes the type of a neighbouring polygon (Subdivision
   neighbours → Subdivision, MP19); floating points give Face (MP14).
   With an explicit type the new polygon gets that type even beside other
   types (MP23).
5. **Explicit types:** Subdivision (MP07), Catmull-Clark (MP12), Spline
   Patch (MP11b), Face (MP10) make one closed polygon like Auto. Curve
   makes an open curve in selection order starting at the first point
   (MP08); Polyline likewise, as a line polygon (MP13).
6. **Close Curve** on a Curve repeats points so the curve closes on itself:
   four points 0,1,2,3 give the control list 2,3,0,1,2,3,0 (MP09). On Face
   it changes nothing (MP10).

### From edges (edge mode)

7. **A closed edge loop** (the border of a hole) is filled with one
   polygon, wound to agree with the surrounding polygons, selection order
   irrelevant (MP17).
8. **Two separate edges** (facing edges of two quads one unit apart) make
   nothing and give no message (MP18). See Q-MS-02.

### After Make

9. The new polygon is appended last and selected (added to any existing
   polygon selection); in vertex mode the point selection is cleared; in
   edge mode the edges stay selected. The selection mode doesn't change.
10. Nothing usable selected (edge mode, no edges): the command failed with
    a "select geometry" message and changed nothing (first-run log only;
    the case file was replaced by the two-edge MP18).

## Split Polygon

Uses selected points (vertex mode) to cut polygons along new edges.

11. **Two points of one polygon, not adjacent:** the polygon is replaced
    by two polygons sharing the new edge (SP01 quad → two triangles, SP03
    hexagon → two quads, SP07 pentagon → quad + triangle). Both keep the
    original facing.
12. **Order of the two pieces:** with the cut points at positions i < j in
    the original corner list, the first new polygon is i, j, j+1, …, i−1
    (wrapping) and the second is i, i+1, …, j (SP03 [0,3,4,5] then
    [0,1,2,3]; SP07 [1,3,4,0] then [1,2,3]; SP12 [2,4,5,0,1] then
    [2,3,4]). The original is removed and the two pieces are appended at
    the end; later polygons move down one slot (SP05).
13. **Selection order doesn't matter** (SP07 = SP08).
14. **Adjacent points** (already an edge) change nothing (SP02); points
    with no polygon in common change nothing (SP06); neither gives a
    message.
15. **A chain across polygons** cuts every polygon that holds two
    consecutive points of the chain: 0,4,8 across a 2×2 grid splits both
    diagonal quads (SP05).
16. **Three or more points in one polygon** make only one cut, between the
    two points earliest in the polygon's corner list (0 and 2 for 0,2,4 on
    a hexagon, in every selection order: SP04, SP10, SP13). **Decided for
    Operandi:** cut between each consecutive pair in selection order instead
    (Q-MS-03); SP04, SP10 and SP13 are Modo reference only.
17. **Fewer than two points selected:** fails with a message ("two or more
    vertices must be selected"), nothing changes (SP09).
18. The point selection is kept; no polygons are selected after.

## Open questions and decisions

- **Q-MS-01** Corner list start (last-selected point) — Operandi may start
  anywhere; cases compare up to rotation. Confirm that's acceptable.
- **Q-MS-02** The guide says Make with two selected edges builds a quad
  between them; scripted edge selection of two separate edges built
  nothing (MP18). Needs a hands-on check (select two
  facing edges, press P).
- **Q-MS-03** (decided below) Split with 3+ points on one polygon makes one cut
  (rule 16). Sane alternative: cut between each consecutive pair in
  selection order (the guide's "in succession"). Provisional under "sane
  over bug-like": open, since Modo's rule is consistent, just limited.
- **Q-MS-04** Make from points of different meshes; Make with points that
  repeat; Spline Patch with other than 4 points (guide: needs 3 or 4).
- **Q-MS-05** Split of non-planar or concave polygons where the new edge
  falls outside the polygon.

## Decided (2026-11-05)

- **Q-MS-03 → decided:** with three or more points on one polygon, cut
  between each consecutive pair in selection order (the guide's "in
  succession"); Modo's single cut is not copied.

Polygon type names in the cases: face = Face, subdiv = Subdivision, psubdiv = Catmull-Clark, spatch = Spline Patch, curve = Curve, line = Polyline/line, bspline = B-Spline, onePoint = one-point polygon.
