# Selection inference GUI check

Spec: specifications/selection-inference-2026-11-15. Use a 3×3 plane and a
cube; do each step on a SubD copy too.

1. **SI-G01 Vertex Bevel from polygons.** Polygon mode, select the middle
   polygon, pick Vertex Bevel from its button and drag. Its four corners
   bevel; the status line says "Vertex Bevel on 4 points from 1 polygon";
   the corners show as selected points on the surface while the tool is
   open. Drop: still Polygon mode, the polygon still selected. Switch to
   Vertex mode: the cut points are selected. Cmd+Z undoes it in one step.
2. **SI-G02 Edge Bevel from polygons.** Select two neighbouring polygons,
   pick Edge Bevel: only their outline bevels, not the edge between them.
   Select every polygon of the cube: every edge bevels and the status says
   "(no outline)".
3. **SI-G03 Edge Bevel from points.** Vertex mode, two neighbouring points:
   Edge Bevel bevels the edge between them.
4. **SI-G04 Polygon tools refuse.** Vertex or Edge mode, pick Bevel: the
   status says "Bevel needs polygons: switch to Polygon mode, or Convert
   the selection".
5. **SI-G05 Commands.** Polygon mode, middle polygon: Join Averaged, Align
   and Set Position (Vertex panel) work on its corners. Edge mode, nothing
   selected: Set Position moves every point.
6. **SI-G06 Keys still pick by mode.** B in Polygon mode is still Bevel,
   in Edge mode Edge Bevel.
