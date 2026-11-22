# Polygon commands

**CMD-P-01 Triple splits along a diagonal inside the polygon.** Concave
quad (0,0) (2,0) (1,0.5) (1,2) (point 2 is the reflex corner): both Quality
and Strip give `[2,3,0]`, `[0,1,2]` (the diagonal through the reflex
corner). Non-planar quad with (1,1,0.4): `[1,2,3]`, `[0,1,3]`. Concave
pentagon (0,0) (2,0) (2,2) (1,1) (0,2): `[1,2,3]`, `[0,1,3]`, `[0,3,4]`. All
results stay selected. (Triple row: "compound, self-intersecting and broader
non-planar polygons".)

**CMD-P-02 Quadruple made no change** on two coplanar triangles forming a
square, on a 4-triangle fan around a centre point, or on three triangles;
selection unchanged, no message. *(Its trigger conditions are open:
CMD-Q01.)*

**CMD-P-03 Spin Quad on quads of different sizes** (`[0,1,4,3]` 1 wide,
`[1,2,5,4]` 2 wide) turns the shared edge: result `[0,1,2,3]`, `[3,2,5,4]`
(same points).

> v2 clarification: any no-change interpretation of CMD-P-04 is superseded by CMD-B3-04. The original oracle already records a connected face winding correction; its numeric data is unchanged.

**CMD-P-04 Align made no change** on `[0,1,4,3]` (+Z) beside `[1,4,5,2]`
(-Z) and a separate `[6,9,8,7]`. *(CMD-Q02.)*

**CMD-P-05 Merge of a ring with a hole** (G3 without its centre polygon, 8
polygons) gives one polygon that walks the outer border and the inner hole
joined by a doubled bridge: `[0,1,2,3,7,11,15,14,10,6,5,9,10,14,13,12,8,4]`
(points 10 and 14 appear twice). No refusal.

**CMD-P-06 Merge of a non-planar block** (2×2 quads with the centre point
raised to z 0.3) gives one octagon of the 8 border points; the interior
point is deleted (9 → 8 points).

**CMD-P-07 Collapse** turns a polygon region into one point at its centre:
G3 polygon 4 → point (1.5, 1.5); polygons 4 and 5 → one point (2, 1.5).
Neighbouring quads become triangles where they lose an edge. Selection
after: nothing.

**CMD-P-08 Unify** removes duplicates of the same point set regardless of
winding or start corner: `[0,1,2,3]`, `[0,3,2,1]`, `[1,2,3,0]` → only
`[0,1,2,3]` (Force on or off). (Unify row: "conflicting attributes" not
tested.)

**CMD-P-09 Remove** deletes the polygon and keeps its points (G3 polygon 4:
16 points, 8 polygons).
