# Sculpt respects the selection: GUI checklist (2026-11-22)

1. Sphere, Polygon mode, select a band of polygons, pick Push: strokes across the band raise only the band; the border is a hard step. Cmd+Z undoes the stroke in one step and the band stays selected.
2. Same with Vertex mode (a few points) and Edge mode (an edge loop).
3. Shift-drag (smooth) and Move on the band: only selected points move.
4. Symmetry X on, select polygons on +X only, stroke there: the -X side stays flat.
5. Drop the selection (nothing selected) or switch to Items: the brush sculpts everywhere again.
6. Two meshes selected in Items, switch to Polygons, select polygons on one: strokes over the other do nothing.
7. Selection stays drawn while sculpting (2026-11-23): Polygon mode, select a band, pick any brush (Push, Smooth, Move, Mask). The band keeps its selected tint and orange outline the whole time the brush is on, with no pre-highlight under the pointer; during a Move stroke the tint and outline travel with the surface. Edge mode (a loop) and Vertex mode (a few points) keep the selected edges / points drawn the same way. A paint tool (Paintbrush) still hides the highlight. No UIC rule covers viewport selection colours; the look is the views' own Show Selections look, and the stroke never changes the selection (UIC-243).
