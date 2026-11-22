# Forum fixes (Loop Slice on polygons, Edge Bevel after hardening): GUI check

Loop Slice (LS-75, LS-Q19):

1. New scene, add a cube with 4 segments each way. Polygon mode, click one
   side polygon.
2. Start Loop Slice with Slice Selected off: a loop runs right round the
   cube through that polygon.
3. Undo, tick Slice Selected, start again: only that polygon is cut, across
   its first side; its neighbours take one point each.
4. Select an L of three polygons, Slice Selected on: the cut turns the
   corner. A 2x2 block gets a closed loop round its centre.
5. With Loop Slice live, click the middle of a polygon: status says "click
   an edge" (unchanged, LS-Q19 asks about it).

Edge Bevel with a vertex normal map (EBC-Q13):

1. New scene, add a cube with 3 segments each way. Polygon mode, select the
   middle polygons of two opposite sides, Bridge with its defaults: a
   tunnel.
2. Edge mode, select the four edges round one tunnel mouth, Vertex Normal
   Tools > Harden (Edges).
3. Edge Bevel those four edges and drag: the bevel appears (before, nothing
   happened). Drop, Undo, Redo.
4. Select the four edges running along the tunnel, Edge Chamfer: works too.
5. Save, reopen: the bevel and the shading are the same.
