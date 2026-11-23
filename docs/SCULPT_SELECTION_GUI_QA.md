# Sculpt respects the selection: GUI checklist (2026-11-22)

1. Sphere, Polygon mode, select a band of polygons, pick Push: strokes across the band raise only the band; the border is a hard step. Cmd+Z undoes the stroke in one step and the band stays selected.
2. Same with Vertex mode (a few points) and Edge mode (an edge loop).
3. Shift-drag (smooth) and Move on the band: only selected points move.
4. Symmetry X on, select polygons on +X only, stroke there: the -X side stays flat.
5. Drop the selection (nothing selected) or switch to Items: the brush sculpts everywhere again.
6. Two meshes selected in Items, switch to Polygons, select polygons on one: strokes over the other do nothing.
