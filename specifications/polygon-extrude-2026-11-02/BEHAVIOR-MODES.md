# Modes and defaults

**PE-M-01 Defaults after Reset:** Shift X/Y/Z 0, Segments 1, UVs U, Mode
Auto. Modes accepted: Auto, Thick, Shift.

**PE-M-02 Shift is one vector in item space applied to every extruded
point** (not per-point normals). Quad, Shift Z 1: new points at z = 1.
Shift (0.5, 0, 1) gives new points (0.5,0,1), (1.5,0,1), (1.5,1,1), (0.5,1,1).

**PE-M-03 Thick (and Auto on an isolated region) builds a closed solid.**
Quad, Shift Z 1, Thick: 8 points, 6 polygons, in this order: four walls
`[6,2,3,7]`, `[5,1,2,6]`, `[4,0,1,5]`, `[7,3,0,4]`, then the source polygon
reversed `[0,3,2,1]` (bottom cap), then the new cap `[4,5,6,7]`. Both caps
are selected after. Two adjacent quads give 12 points, 10 polygons; both
source polygons are kept reversed (`[0,3,4,1]`, `[1,4,5,2]`), the two new
caps follow each, and all four caps are selected.

**PE-M-04 Shift mode builds walls and a moved cap only.** Quad, Shift Z 1,
Shift mode: 8 points, 5 polygons: the same four walls, then `[4,5,6,7]`;
the source polygon is gone. Only the new cap is selected.

**PE-M-05 Auto mode:** on an isolated region (Quad, Two, Bowtie, Cross) it
behaves like Thick. On a region connected to unselected polygons (Fan) it
behaves like Shift: no bottom cap (BEHAVIOR-NONMANIFOLD PE-N-01). *(That the
choice depends on whether the selection has unselected neighbours is
**inferred** from these cases: PE-Q01.)*

**PE-M-06 Extruding against the normal reverses the walls so the solid faces
outward.** Quad, Shift Z -1, Auto: walls `[6,7,3,2]`, `[5,6,2,1]`,
`[4,5,1,0]`, `[7,4,0,3]`; the source polygon keeps its order `[0,1,2,3]`;
new cap `[4,7,6,5]`.

**PE-M-07 Segments** split the walls evenly: Shift Z 1.5, Segments 3 →
points at z = 0.5, 1.0, 1.5; 16 points, 14 polygons (12 wall quads, bottom
cap, top cap). Selection: both caps.
