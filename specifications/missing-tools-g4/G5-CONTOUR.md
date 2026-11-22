# Missing tools, group 5: Contour (v1, 2026-11-21)
Behaviour only. REA-assisted: no. Topology tab tool: replaces operandi.notBuilt "Contour".
What it does: drag a line across the background mesh in the viewport. A cutting plane through that line (and straight into the screen) slices the background surface, and the cut outline is added to the active mesh as a new curve (or line, or face) loop with an even number of points. The retopology way to lay down a ring around an arm or leg in one stroke.

Settings (defaults):
Start X/Y/Z, End X/Y/Z = 0: the drawn line's ends (world)
Slice Direction X/Y/Z = 0 (from the view): the plane's second direction. Drawn in a viewport it's the view direction; typed, use these
Contour Type = Curve: Lines, Curve or Face
Automatic = off: on = point count chosen from the surface (MT13-06)
Segments = 10: point count when Automatic is off
Uniform Span = on: even spacing along the loop
Slicing Side = Both: Positive, Negative, Both: which side of the slice direction to keep
Cull Occluded Open Curves = on: open pieces hidden from the viewer are dropped
Intersect = off: on = slice the active mesh itself instead of the background
Select New Polygons = on
Angle Snap / Angle = off / 45°: snap the drawn line's angle on screen
Fast Slice = on: off = preview the cut live while dragging; on = only on release

Rules:
- MT13-01 The plane contains the drawn line and the slice direction. Line (−1,0,0)->(1,0,0), direction (0,0,1) -> plane y = 0 (CT-1); line raised to y = 0.25 -> plane y = 0.25 (CT-6).
- MT13-02 The plane cuts the background meshes' surfaces (visible, inactive meshes). Points land exactly on the background surface (CT-1: every point at radius 0.5 within 0.0001).
- MT13-03 Extent: the cut only reaches as far as the drawn line: if the line spans the whole object, the full closed loop (CT-1); if shorter, an open piece covering just the stretch under the line (CT-7: line x −0.1..0.1 -> open curve from x = 0.1 to x = −0.1, on the near side). Cull Occluded Open Curves keeps the piece facing the viewer.
- MT13-04 Slicing Side Positive keeps the part of the loop on the + side of the slice direction (CT-8: open curve over the +Z half). Negative the other half. Both = full loop.
- MT13-05 Point count/spacing (Automatic off, Uniform Span on): a closed loop gets exactly Segments points (CT-1: 10 points, 36° apart); an open piece gets Segments + 1 (CT-7: 11 points, ends on the two line ends). Evenly spaced by length along the cut.
- MT13-06 Automatic on: closed cut of a 24-sided globe sphere gave 25 evenly spaced points (CT-4). Reading: count = background edges crossed + 1. PROVISIONAL.
- MT13-07 Uniform Span off: CT-5 (Segments 4) gave 4 points 90° apart. Reading: off = even steps of the raw cut's point index instead of by length. PROVISIONAL.
- MT13-08 Curve = one curve polygon through the points (closed when the loop is closed); Lines = one line polygon (polyline, closed by repeating the first point); Face = one face polygon.
- MT13-09 Q-CT-1 PROVISIONAL: build Curve only; grey out Lines and Face, tooltip "Not decided yet". (Recommendation pending Jason: apply MT13-05 spacing to all three.)
- MT13-10 New polygon added to the active mesh (empty mesh fine) and selected (Select New Polygons on). Background untouched.
- MT13-11 Closed loop starts at the cut point nearest the drawn line's start.
- MT13-12 Nothing under the line: nothing made, message "Contour: the line doesn't cross a background surface."
- MT13-13 Intersect on: cut the active mesh's own surface instead. PROVISIONAL.
- MT13-14 Drag in the viewport to set Start/End (projected onto the background, or the work plane when nothing is under the cursor); slice direction = view direction. Release applies (Fast Slice on). Dragging again replaces the last contour until the tool is dropped, like the slice tools.

Cases (background = Sphere Item, globe 24 sides x 12 segments, radius 0.5, at origin; active mesh empty; line (−1,0,0)->(1,0,0), direction (0,0,1) unless noted):
CT-1 defaults: one closed curve, 10 points on the y = 0 circle r 0.5, 36° apart
CT-2 Lines / CT-3 Face: greyed for now (Q-CT-1)
CT-4 Automatic: closed curve, 25 points, 14.4° apart
CT-5 Segments 4, Uniform off: closed curve, 4 points 90° apart
CT-6 line at y = 0.25: closed curve, 10 points on circle y = 0.25, r 0.4330
CT-7 line x −0.1..0.1: open curve, 11 points from (0.1, 0, 0.4868) to (−0.1, 0, 0.4868), even spacing
CT-8 Slicing Side Positive: open curve, 11 points over the +Z half, from −15° to 195° (end overshoot PROVISIONAL; test the +Z half and count only)
