# Vertex commands

**CMD-V-01 Merge, fixed distance 0.01** merges points closer than 0.01 to
their midpoint: two quads with facing edges at x = 1 and x = 1.004 → points
at (1.002, 0) and (1.002, 1); 8 → 6 points. **Auto** range did not merge
them (no change).

**CMD-V-02 Join** of points 5, 6, 9 on G3: with Average on they become one
point at their centroid (1.333333, 1.333333); with Average off, at the last
selected point's position (1, 2). Either way 16 → 14 points, 9 → 8
polygons (the polygon between them collapses).

**CMD-V-03 Collapse** of point 5 (interior, valence 4) removed the four
polygons around it and left 12 points and 5 polygons (exact lists in the
oracle; the rule behind which points go is open: CMD-Q07).

**CMD-V-04 Remove** of point 5 drops it from its polygons, which become
triangles (16 → 15 points, still 9 polygons).

**CMD-V-05 Split** of point 5 gives each of its four polygons its own copy
(16 → 19 points).

**CMD-V-06 Align (all axes)** moves the selected points onto the straight
line from the first to the last selected point: (1,0.2,0) → (1, 0.133333, 0),
(2,-0.1,0.3) → (2, 0.266667, 0) on the line from (0,0,0) to (3,0.4,0). Axis
Y changes only Y: (2,-0.1,0.3) → (2, 0.266667, 0.3).
