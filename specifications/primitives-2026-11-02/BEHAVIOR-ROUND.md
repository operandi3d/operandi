# Sphere, Cylinder, Cone, Ellipsoid

**PRIM-S-01 Sphere Globe** 8 sides × 4 segments: 26 points, 32 polygons
(quads plus triangle fans at the poles). Axis X gives the same counts.

**PRIM-S-02 Sphere modes (SP-Q09/Q10):** Quad Ball level 0 → 8 points,
6 quads: a cube with corners at (±0.654591, ±0.654591, ±0.654591) and the
same UV cross layout as the Cube; level 1 → 26 / 24. Tessellated level 0 → 12 points,
20 triangles (icosahedron); level 1 → 42 / 80.

**PRIM-Y-01 Cylinder caps (CYL-Q02/Q12),** 8 sides, 1 segment: Single →
16 points, 10 polygons; None → 16 / 8; Quads → 34 / 32; Radial → 18 / 24
(a centre point and a triangle fan per cap). Axis X: 16 / 10.

**PRIM-Y-02 Edge Weight** is refused with face polygons ("only applies for
subdivision surfaces"); the cylinder (8 sides, 2 segments) was still made:
24 points, 18 polygons.

**PRIM-K-01 Cone,** 8 sides: 9 points, 9 polygons: an octagonal base
`[0,7,6,5,4,3,2,1]` at y = -0.75 and eight triangles to the apex (0,0.75,0);
base point 0 at (0,-0.75,-0.5), point 1 at (-0.353553,-0.75,-0.353553),
continuing around through -X, +Z and +X.

**PRIM-K-02 Odd-sided Quads cap (CA-Q01):** 5 sides, bottom cap Quads →
11 points, 12 polygons: an inner ring of 5 points at half radius, 5 outer
quads, and the inner pentagon split into one triangle `[5,9,6]` and one quad
`[9,8,7,6]`. 8 sides → 18 / 20.

**PRIM-K-03 Zero and negative height (CA-Q03/Q04):** size Y 0 → 8 points,
1 polygon (the base disc only). Size Y -0.75 → the cone is mirrored (base at
y = +0.75, apex at -0.75) with every polygon's winding reversed so it still
faces outward.

**PRIM-E-01 Ellipsoid** 8 × 4: 26 points, 32 polygons. With UV
Compensation on, size Y 0 and size X -0.5 still give 26 / 32; the flattened
case has all points at y = 0 and pole triangles with UVs like
(0.0625,0), (0.125,0.25), (0,0.25) (EA-Q02; full lists in ORACLES.json).
