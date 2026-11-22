# Vertex Merge and Join: GUI checklist

Spec: vertex-tools-2026-11-14 v1.3 (VX-M01..M06, VX-C01, VX-B11). QA app,
fresh scene, Vertex mode.

- **VMQ-01 Merge Tool fields.** Vertex panel ▸ Merge: Tool Properties shows
  Distance 0.001, Keep 1-Vertex Polygons (off), Test Morph Map Positions
  (off).
- **VMQ-02 Average.** Make three points close together (Create or move
  them), select all three, Merge Tool, Distance 0.1: they become one point
  in the middle of the three, and nothing stays selected.
- **VMQ-03 Keep 1-Vertex Polygons.** Select two corners of a triangle,
  Merge with a Distance that reaches: the triangle disappears. Cmd+Z, tick
  Keep 1-Vertex Polygons, repeat: a line is left.
- **VMQ-04 Merge dialog.** Vertex panel ▸ Merge…: Range, Distance (Fixed
  only), Keep 1-Vertex Polygons, Test Morph Map Positions, Merge
  Discontinuous UV Values (on). OK runs it once; Cmd+Z undoes it.
- **VMQ-05 Join dialog.** Vertex panel ▸ Join…: Keep 1-Vertex Polygons,
  Join Discontinuous UVs. OK joins at the last selected point. Join
  Averaged runs at once.
- **VMQ-06 Short-edge bevel.** Plane 3×3, move one neighbour of an interior
  point to 0.15 m away, Vertex Bevel Inset 0.2 Round Level 2: the cut stops
  at the neighbour and the cap's inside leans toward it smoothly, with no
  folded quads.
