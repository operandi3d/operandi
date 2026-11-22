# G7 Patch Curves (finish Draw Patch)

REA-assisted: no. Sources: the reference's public user guide ("Patch Curves" page), its plain-text tool option list, and hands-on tests in the reference (scripted scene, synthetic clicks, mesh read back by script), 2026-11-21. No screenshots, internal names or vendor files are carried over.

## What Operandi already has
Draw Patch on the Create and Curve tabs: pick four curves that share their ends, get a frozen grid of quads (Perpendicular x Parallel), Flip, Make UVs. Arc-length spacing only. This spec covers what is missing.

## PC-1 Duplicate-tab button
Add a Patch Curves button to the Duplicate tab of the toolbox. It runs the same tool as Draw Patch (same options, same state). The reference has this one tool under three buttons: Create > Patch, Curve tab > Patch, and Duplicate > Patch Curves.

## PC-2 Picking and closing the loop (hands-on)
- With the tool on, every curve shows a handle at its middle.
- Click the curves one after another around the loop. Each picked curve's handle turns highlighted; the most recent pick is drawn larger.
- The patch is made when you click the first curve again, closing the loop. Four curves = five clicks (A, B, C, D, A). Three curves = four clicks (A, B, C, A). Operandi today builds on the fourth pick; change it to wait for the closing click, so the three-curve case works the same way.
- Curves must share end points (merged). Ends that only touch build nothing (already in BEHAVIOR-DRAW PCH-03).
- Picking direction (clockwise or not) does not matter.

## PC-3 Three curves (hands-on; closes PCH-Q01)
Three curves meeting end to end make a patch too. It is a four-sided patch with one side shrunk to a single corner. When frozen at N x N, the row of cells touching that corner is triangles (N of them) and the rest are quads. Observed at 16 x 16: 240 quads + 16 triangles, no zero-area polygons, and the collapsed corner is one shared point (not N copies).
Build it as a Coons patch whose fourth boundary is the corner point repeated, then merge the repeated corner and turn the quads that touch it into triangles.
PROVISIONAL (PC-Q1): which corner collapses. Use the corner opposite the first picked curve until checked.

## PC-4 Perpendicular and Parallel (hands-on)
With picks bottom, right, top, left (bottom first), Perpendicular 4 and Parallel 6 gave a 4 x 6 grid of 24 quads:
- Parallel = divisions along the first picked curve (6 along it).
- Perpendicular = divisions along the curves that meet it (4 across).
Check Operandi uses the same pairing.

## PC-5 Frozen result (hands-on)
- One quad per grid cell, Perpendicular x Parallel quads.
- The four corner points are the curves' own corner points (reused, not copied). For 4 x 6: 31 new points = 5 x 7 grid minus the 4 corners.
- Points along the patch edges are new points, not the curves' own middle points, even where they land in the same place.
- With Save Boundaries on, the curves stay.
- Oracle: curves (each a 3-point smooth curve that passes through its points):
  - bottom (−2,0,−1) (−1.5,0.3,−1.2) (−1,0,−1)
  - right (−1,0,−1) (−0.8,0.3,−0.5) (−1,0,0)
  - top (−1,0,0) (−1.5,0.3,0.2) (−2,0,0)
  - left (−2,0,0) (−2.2,0.3,−0.5) (−2,0,−1)
  The grid centre lands at (−1.5, 0.6, −0.5) within 0.001. That is the plain bilinear Coons result: the sum of the four mid-points over 2, minus the sum of the corners over 4.

## PC-6 Spacing: Type 1 / Type 2 (options; not tested)
Two drop-downs, one per direction, choices Length (default) and Knot.
- Length: divisions evenly spaced by arc length. Operandi already does this.
- Knot: divisions spaced evenly in the curve's own parameter, so each span between control points gets an equal share of the divisions.
- Type 1 goes with Perpendicular and Type 2 with Parallel.
PROVISIONAL (PC-Q2): the Knot rule is read from the option name and the guide, not measured.

## PC-7 Freeze off: a live patch (DECIDED, Jason 2026-11-21)
PC-7 DECIDED (Jason, 2026-11-21): build live patches. New "patch" polygon kind:
- A patch polygon lists only its 3 or 4 corner points. Its surface is worked out from the curves between those corners, using the same Coons maths as PC-5 and PC-3. Each curve is found by the pair of corners it joins.
- It draws shaded like a surface: tessellated at the item's patch subdivision level, 16 x 16 cells by default. It updates live when a boundary curve or corner moves.
- Delete a boundary curve and the patch can't be worked out: it draws nothing and is flagged broken. PROVISIONAL (PC-Q5).
- The general Freeze command turns a patch polygon into quads at that subdivision level (16 x 16, plus triangles at a collapsed corner).
- Render: tessellated the same way. Save: kept as a patch polygon. USD export: write the tessellated mesh.
- Tool: with Freeze off (the default), closing the loop leaves one patch polygon. Perpendicular and Parallel are greyed out.

Tests: patch polygon round-trips save/reopen; moving a boundary curve point reshapes the drawn surface; Freeze gives 16x16 quads (+ N triangles at a collapsed corner); deleting a boundary curve flags it broken and draws nothing; USD export writes the tessellated mesh; Cycles render gets the tessellation; command_contract + monkey safety for the new polygon kind; speed guard for live redraw.

## PC-8 Facing
Keep Operandi's rule (face the viewer, Flip reverses). That is a deviation already in main, so leave it.

## PC-9 Edit modes: NOT in this batch.

## Tests to add
1. Five clicks on a 4-curve loop build; four picks alone build nothing.
2. A 3-curve loop builds N x N with N triangles at one corner and one shared corner point.
3. 4 x 6 oracle: 24 quads, 31 new points, corners reused, centre (−1.5, 0.6, −0.5) ± 0.001.
4. Parallel counts along the first picked curve.
5. Duplicate tab shows Patch Curves and it opens the same tool.
