# Primitives created with Symmetry on

> v2 draft: The general doubling statement below is superseded for a center exactly on the plane by SYM-B2-05; the stated offset-center example remains valid.

**SYM-PR-01** A primitive created while Symmetry is on is created twice in
the same mesh: once as specified and once mirrored across the plane. Setup:
empty mesh, Symmetry X, Cube center (1,0,0), size 1, 1 segment each axis.
Result: 16 points, 12 polygons (Symmetry off: 8 and 6). (SYM-O-07.)

**SYM-PR-02** Order: the specified copy comes first exactly as it would
without Symmetry (points 0–7, polygons 0–5). The mirrored copy follows:
point 8 + i is point i with x negated; polygon 6 + k uses the mirrored
points of polygon k with its winding reversed, keeping the first corner:
`[a, b, c, d]` becomes `[a', d', c', b']`. Normals therefore stay outward.

**SYM-PR-03** The same holds for the Ellipsoid: center (1,0,0), size 0.5,
other settings at their reset values: 532 points and 576 polygons versus 266
and 288 with Symmetry off, spanning x -1.5…1.5. (SYM-O-08; EA-Q03.)

**SYM-PR-04** Selection after creation: nothing selected in the mesh.
