# Odd input

**PE-O-01 Zero shift.** Auto/Thick: no new points; a reversed copy of the
polygon is added (`[0,1,2,3]` and `[0,3,2,1]`, 2 polygons) and both are
selected. Shift mode: walls are built with new points coincident with the
old ones (8 points, 5 polygons), new cap selected.

**PE-O-02 Sideways shift (in the polygon's plane) merges coincident points.**
Quad, Shift X 1 (Auto or Thick): 6 points, 6 polygons: new points only at
(2,0,0) and (2,1,0); the points that would land on (1,0,0) and (1,1,0) reuse
the existing ones, and two walls become triangles (`[1,4,0]`, `[5,2,3]`).
Shift mode: 6 points, 5 polygons. No message.

**PE-O-03 Empty selection extrudes every polygon** (Quad, nothing selected,
Shift Z 1: same as PE-M-03). Selection after: both caps.

**PE-O-04 Non-planar polygon (EX-Q01).** Quad with point 2 at (1,1,0.3),
Shift Z 1: every point moves by the same (0,0,1) (point 2's copy at
(1,1,1.3)); 8 points, 6 polygons, as PE-M-03. No refusal.
