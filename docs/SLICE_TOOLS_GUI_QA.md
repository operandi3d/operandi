# Slice tools (Tier 1A): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi slice-tools <commit>.app`), on a throwaway scene.
Each line: do this → expect that.

## Edit tab

1. Toolbox tabs read Create, Select, Edit, Vertex, Edge, Polygon. Edit
   shows a Slice heading with Polygon Slice (Shift+C), Edge Slice (G),
   Loop Slice (Alt+C) as text rows with their shortcuts.
2. Each row starts its tool (lit) and shows Tool Properties; a second click
   drops it.

## Loop Slice (Alt+C)

1. Cube, Segments 3. Edge mode, select one edge, Alt+C → a loop appears at
   50 % across the ring; the slider bar sits at the top-left of the view
   (200 pt long, 20/50 from the corner) with one triangle.
2. Drag the triangle → the loop follows live; release; Cmd+Z → back to
   50 %, tool still on. Drop (Q), Cmd+Z → the whole slice goes; Edit menu
   said "Undo Loop Slice".
3. Count 3, Mode Uniform → three evenly spaced loops; Mode Symmetry, drag
   the left triangle → the right one mirrors it.
4. Edit Add: click the bar → a new slice there. Edit Remove: click a
   triangle → it goes (the last one stays).
5. Drag the bar's right dot → the bar gets longer; the left dot moves its
   start. The Length / Slider X / Slider Y fields follow.
6. Nothing selected, Alt+C → no loop; click an edge → its ring is cut;
   Shift+click another edge → a second ring.
7. Polygon mode, select two neighbouring polygons → the ring through them;
   Slice Selected keeps the cut inside them.
8. Cylinder, Preserve Curvature on, slice across the side ring → new points
   sit on the round surface; Tension 0 % → flat.
9. Greyed at the start, each with a tooltip: Keep Quads (until Slice
   Selected), Reverse Direction, Reverse Inset, Keep Aspect (until a
   Profile is picked), Tension, Cap Sections and Gap (until Split). Split
   and Profile are never greyed.
10. 3x3 grid, select the two bottom-left squares, Alt+C, Slice Selected →
    the third square becomes 5-sided; Keep Quads changes nothing there.
11. Split → the strip comes apart along the loop (move a point on one side:
    the other stays). Gap 0.1 → two loops at 45 % and 55 %. Cube, edge
    ring round four sides, Split + Cap Sections → two closed halves.
12. Profile T Slot → a T-shaped slot cut into the bottom row, centred;
    Current, Count, Uniform, Position and Inset grey out. Keep Aspect off → Inset sets the depth.
    Reverse Inset → a T-shaped ridge. Saw Notch + Reverse Direction → the
    slope flips sides.

## Edge Slice (G)

1. Grid or cube, press G. Click an edge, then an edge of the same polygon →
   the polygon is cut between them; keep clicking across neighbours.
2. Press on an edge and drag along it → the point slides in 0.5 % steps;
   the percent shows beside it (Show Position). Show Distance → metres to
   both ends.
3. Drag an existing point → it moves along its edge, the cut follows.
4. With three or more points, hover the first point → it turns the
   closing colour; click → the loop closes.
5. Ctrl+click → the point lands at the middle (Split at Middle does it for
   every click).
6. Click inside a polygon next to the last point (Inside Polygons on), then
   an edge on its far side → the cut bends through that inner point.
7. Split Polygons off → points only, no new edges. Multi Slice on → dashed
   line from the last point to the pointer.
8. Cmd+Z inside the tool removes the last click; Esc cancels everything.

## Polygon Slice (Shift+C)

1. Cube, nothing selected, Top view, Shift+C. Drag a line across the cube
   → every polygon the line crosses is cut, live.
2. Drag an end handle or the line → the cut moves. Shift+drag → a second
   cut, the first stays. Middle-click elsewhere → another cut of the same
   length and direction there.
3. Start/End fields follow the line; typing values moves it.
4. Infinite off (default): only polygons along the drawn line are cut;
   Infinite on: the whole plane cuts.
5. Snapping on (X) or Ctrl held → the line snaps to 45° steps.
6. Split on → the two sides come apart (try Move on one side); Cap
   Sections → both halves closed; Gap 0.1 → a gap between them; Offset
   Side moves it.
7. Fast Slice → the line draws while dragging, the cut appears on release.
8. Select a few polygons first → only they (and their pieces) are cut.
