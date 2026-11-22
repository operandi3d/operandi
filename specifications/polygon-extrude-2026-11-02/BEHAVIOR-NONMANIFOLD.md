# Non-manifold and mixed input

**PE-N-01 Polygon on an edge shared by three polygons (EX-Q01).** Fan,
polygon 0 only, Shift Z 0.5 (Auto): 12 points, 7 polygons. No bottom cap. A
wall is built along the shared edge 0–1 too (`[11,1,0,8]`), so edge 0–1 then
belongs to three polygons plus that wall. The two unselected polygons keep
their points. New cap `[8,9,10,11]` selected.

**PE-N-02 Two of the three.** Fan, polygons 0 and 1, Shift Z 0.5: 14 points,
9 polygons: the two are extruded as one region (one shared wall-free edge),
no bottom caps, the third polygon `[0,1,7,6]` unchanged; caps selected.

**PE-N-03 Polygons touching at one point.** Bowtie, both, Shift Z 0.5: the
shared point gets one new point (7 new points in all, 14 points); each
polygon gets four walls and both caps (12 polygons). Caps selected.

**PE-N-04 Polygons facing different ways** use the same shift vector.
Cross, both, Shift Z 0.5: the XY quad is raised to z = 0.5; the YZ quad
(normal along X) is pushed along Z within its own plane, to
(3,0,0.5)…(3,1,1.5). 16 points, 12 polygons.

**PE-N-05 Transformed item: the shift is in world space (EX-Q04 part).**
Quad in an item at (3,0,0) rotated 90° about Y, Shift Z 1: in the mesh's
own coordinates the new points are at x = -1 (world +Z is local -X), so the
extrusion lies in the polygon's plane and merges as in PE-O-02 (6 points,
6 polygons).
