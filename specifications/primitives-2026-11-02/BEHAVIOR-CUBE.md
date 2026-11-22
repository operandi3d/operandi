# Cube

**PRIM-C-01 Default cube** (center 0, size 1, 1 segment, UVs on): 8 points,
6 polygons in this order: -Y `[0,1,2,3]`, -Z `[4,5,1,0]`, +X `[5,6,2,1]`,
+Z `[6,7,3,2]`, -X `[7,4,0,3]`, +Y `[4,7,6,5]`; points from (-0.5,-0.5,-0.5)
in the order of PRIM-O-cube-default. A "Texture" UV map is created.
Nothing is selected afterwards.

**PRIM-C-02 Default UV layout (SP-Q13):** a cross of six equal square tiles
on a 4 × 3 grid of the unit square (tile 0.25 wide, 0.333333 tall): -X, +Z,
+X, -Z left to right in the middle row (u 0–0.25, 0.25–0.5, 0.5–0.75,
0.75–1; v 0.333333–0.666667), -Y below +Z (v 0–0.333333) and +Y above +Z
(v 0.666667–1). Corner order per face: ORACLES.json.

**PRIM-C-03 UV Gaps 0.1** shrinks the whole layout toward the centre of the
unit square, leaving a 0.1 border: every coordinate c becomes
0.5 + (c − 0.5) × 0.8 (e.g. 0.25 → 0.3, 0 → 0.1). Tiles still touch each
other. *(Only one value was measured; the factor 1 − 2 × Gaps is inferred.)*

**PRIM-C-04 UV Compensation** keeps tiles proportional to face size: size
(2,1,1) gives -Y tile u 0.166667–0.5, v 0–0.166667 (2:1), +X tile 0.5–
0.666667 wide; the layout is scaled to fit the longest row into u 0…1.

**PRIM-C-05 Segments** split faces evenly: 2/1/1 → 12 points, 10 polygons.

**PRIM-C-06 Rounding (BOX-Q10):** Radius 0.1 with Radius Segments 1 → 104
points, 114 polygons; Segments 3 → 248 / 266. With **Sharp** off, Segments 3
gave 104 / 114 (the same as Segments 1). Rounding Axis X or Y with Segments
2 → 168 / 182 each. Positions: ORACLES.json.

**PRIM-C-07 Bulge needs a radius:** Bulge 0.2 with Radius 0 was refused
("corner radius must be non-zero") and an unbulged 2×2×2-segment cube was
created (26 points, 24 polygons). *(BOX-Q14: Bulge with a radius not yet
measured.)*

**PRIM-C-08 Zero size makes one polygon:** size Y 0 → 4 points, 1 polygon
`[0,3,2,1]` in the plane y = 0, facing +Y, with UVs (0,1), (0,0), (1,0),
(1,1).

**PRIM-C-09 Negative size is treated as positive:** size X -1 gave exactly
the default cube (BOX-Q08: crossed bounds are not refused numerically).
