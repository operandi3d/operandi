# Junctions and shared points

All with Edge Bevel Inset 0.1 or Edge Chamfer Offset 0.1, Segments 1. Both
tools give the **same positions and polygon shapes** in these cases; only
point/polygon numbering and the selection afterwards differ (exact lists in
ORACLES.json).

**EBC-J-01 One edge.** Box edge 6–7: 10 points, 7 polygons. The edge is
replaced by a flat strip whose long sides lie 0.1 from the edge on each
adjacent face: (±0.5, 0.4, 0.5) on +Z and (±0.5, 0.5, 0.4) on +Y. The end
faces (+X, -X) become pentagons. Selection after: Edge Bevel selects the
strip's four edges; Edge Chamfer selects nothing.

**EBC-J-02 Two edges sharing a point.** L (6–7 and 5–6): 11 points, 8
polygons, no extra corner polygon. The two strips meet at one point on the
top face, (0.4, 0.5, 0.4); the strip of 6–7 runs from (-0.5, 0.5, 0.4) /
(-0.5, 0.4, 0.5) to (0.4, 0.5, 0.4) / (0.5, 0.4, 0.5), and the strip of 5–6
from (0.4, 0.5, 0.4) / (0.5, 0.4, 0.5) to (0.4, 0.5, -0.5) / (0.5, 0.4, -0.5).
The +X face stays a quad with its top edge lowered to y = 0.4. Edge Bevel
selects the 7 new edges; Chamfer nothing.

**EBC-J-03 Three edges at a corner.** corner3: 13 points, 10 polygons. The
corner point is replaced by a triangle with corners (0.4, 0.4, 0.5),
(0.5, 0.4, 0.4), (0.4, 0.5, 0.4); each strip ends on it. Edge Bevel selects
12 edges; Chamfer nothing.

**EBC-J-04 Closed loop.** toploop: 12 points, 10 polygons; the top face
shrinks to a 0.8 × 0.8 square at y = 0.5 and each side face is cut 0.1
below the top.

**EBC-J-05 Corner with more segments.** corner3, Edge Chamfer Segments 2:
20 points, 15 polygons; the corner gets three quads meeting at one point
(0.460948, 0.460948, 0.460948). Edge End Triangle Fan gave identical
output in this case. Edge Bevel Round Level 1 on corner3: 20 points, 15
polygons.
