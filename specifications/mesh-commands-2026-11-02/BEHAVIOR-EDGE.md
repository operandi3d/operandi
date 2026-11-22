# Edge commands

**CMD-E-01 Remove of an interior edge** merges its two polygons into one:
G3 edge 5–9 → hexagon `[4,5,6,10,9,8]`; points kept (Keep on or off made no
difference here).

**CMD-E-02 Remove of a border edge** deletes its polygon's side: G3 edge
0–1 → 14 points, 8 polygons; the corner polygon becomes a triangle and the
edge's two points are removed (renumbered; see oracle).

**CMD-E-03 Collapse** merges the edge into one point at its middle: edge
5–9 → (1, 1.5); the neighbouring quads become triangles.

**CMD-E-04 Join (average)** of two parallel edges 5–9 and 6–10 merges them
into one edge at their average: points (1.5, 1) and (1.5, 2); 14 points,
8 polygons.

**CMD-E-05 Split** (gap 0.1, caps on or off) made no change on an interior
edge of G3. *(CMD-Q03.)*

> v2 clarification: the earlier Percent 25 result describes an out-of-range driving value. UI Percent 25 behavior is specified by CMD-B3-01; the old measurement is retained unchanged.

**CMD-E-06 Subdivide** with Count 2 added **one** point at the middle of
edge 5–9, (1, 1.5); the two neighbouring quads become pentagons. Percent 25
with Count 1 put the point at (1, 1.999). The new point and its two edges
are selected. *(Count and Percent semantics are open: CMD-Q04.)*
