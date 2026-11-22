# Per tool

Selected element and settings for each case; full lists are in ORACLES.json
(ids `UVN-O-<tool>-cont` and `UVN-O-<tool>-seam`). Point and polygon counts
are the same with and without the seam.

**UVN-T-01 Edge Bevel**, Inset 0.2. Continuous: edge 5–9; seam: edge 6–10
(on the seam). 16 → 20 points, 9 → 12 polygons (a strip quad and a triangle
at each end). Points are renumbered: the edge's two original points are
removed and the new points take indices 14–19. Seam: the strip quad takes the
**right (seam) frame on all four corners**, including the two corners on the
left side (u = 1.6 at x = 1.8). Each end triangle has one corner in the left
frame and two corners carrying the original seam point's continuous UV
(0.666667, 0.333333 or 0.666667, 0.666667). Selection after: the new edges
(and their points).

**UVN-T-02 Edge Chamfer**, Offset 0.2, Segments 1. Same edges. 16 → 20
points, 9 → 14 polygons. At each end of the edge Modo made **two coincident
zero-area triangles** (e.g. points 5, 18, 17 all on y = 1) in addition to the
strip quad. Seam: the strip quad's right corners carry the original seam
points' continuous (left) values (0.666667, …), not the right frame; the two
coincident triangles at each end carry different frames (one mostly right,
one left). Selection after: nothing.

**UVN-T-03 Edge Extrude**, Inset 0.2, Shift 0.2. 16 → 22 points (4 inset
points + 2 raised), 9 → 15 polygons. Raised points at z = 0.2 copy the
original edge points' UVs (UVN-R-03). Seam: the two wall quads and the two
triangles on the right side are wholly in the right frame; the two triangles
on the left side wholly in the left frame. Selection after: the raised edge.

**UVN-T-04 Vertex Bevel**, Inset 0.2. Continuous: point 5; seam: point 6.
16 → 19 points, 9 → 10 polygons (one cap quad). Seam: the cap's corners are
in the left frame except the corner inside the right polygon (2.2, 1), which
carries the original point's continuous UV (0.666667, 0.333333). Selection
after: the cap's points and edges.

**UVN-T-05 Vertex Extrude**, Inset 0.2, Shift 0.2. 16 → 20 points, 9 → 13
polygons (four triangles meeting at a raised apex). Each triangle is wholly in
the frame of the polygon it replaced part of; the apex copies the original
point's UV in that frame. Selection after: the apex point.

**UVN-T-06 Grow Quads** (`auto` on), top border edges 12–13 and 13–14
(continuous) or 13–14 and 14–15 (seam: polygon 8 at the top right is the
seam polygon). 16 → 19 points, 9 → 11 polygons; new points at y = 4. Seam:
the quad grown from the seam polygon has its two new corners in the right
frame (x/3 + 1, 4/3) but its two border corners carry the border points'
continuous (left) values, so the new quad's UVs jump by 1 in u across it.
Selection after: nothing. With `auto` off, the new points were placed **on**
the border (y = 3, zero-height quads) in an earlier run.

**UVN-T-07 Fill Quads** (grid off), the 8 border edges of the G4 hole.
25 points (none added), 12 → 15 polygons: the hole is filled with three
quads using the existing border points. Continuous: all corners take the
points' continuous values. Seam: the new quad touching the seam side takes
the seam frame at its two right-hand corners; **and the existing polygon 2
`[2,3,8,7]`, which was not selected, had its corner at point 8 changed from
(1, 0.333333) to (2, 0.333333)**. Selection after: the border edges.

**UVN-T-08 Bridge**, Segments 2, edges 1–2 and 4–7. 8 → 10 points (two
midpoints at x = 1.5), 2 → 4 polygons. Continuous: midpoints (0.5, 0) and
(0.5, 1/3). Seam: midpoints u = 1.0, the average of the two borders' values
(1/3 on the left, 5/3 on the right). Selection after: the two bridged edges.

**UVN-T-09 Chamfer Edit**, Offset 0.1, Segments 2, Source Segments 1,
Depth 100, polygons 1, 4, 7 (the middle column) as the strip. 16 → 20
points, 9 → 12 polygons. The strip's border points move outward by 0.1 and
keep their UVs; new middle points at x = 1.5 get (0.5, y/3) (UVN-R-05). Seam:
same, with the right side keeping the seam frame on the moved points.
Selection after: nothing.
