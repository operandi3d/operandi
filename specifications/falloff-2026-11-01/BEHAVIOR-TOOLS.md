# Topology tools under a falloff

Fixture: a strip of two quads, points 0, 1, 2 at (-1,0,0), (0,0,0), (1,0,0)
and 3, 4, 5 at (-1,1,0), (0,1,0), (1,1,0); polygons [0,1,4,3] and [1,2,5,4].
Falloff: Linear, Start (-1,0,0), End (1,0,0), so weights are 1 at x = -1,
0.5 at x = 0, 0 at x = 1.

**FAL-T-01** Polygon Bevel scales each new point's Shift and Inset by the
weight of the corner it comes from. Both polygons selected:
- Inset 0, Shift 1: new points at z = 1, 0.5, 0 for x = -1, 0, 1. Counts are
  unchanged by the falloff (12 points, 8 polygons), even where the weight is
  0 (the new point then sits on the original corner).
- Inset 0.2, Shift 0: new points (-0.8,0.2), (0,0.1), (1,0), (-0.8,0.8),
  (0,0.9), (1,1) instead of (-0.8,0.2), (0,0.2), (0.8,0.2), …

**FAL-T-02** Edge Bevel scales its width by the weight at the edge. Edge
1–4 (at x = 0, weight 0.5), Inset 0.2: new points at x = ±0.1 instead of
±0.2. Counts unchanged (8 points, 3 polygons).

**FAL-T-03** Vertex Bevel scales its inset the same way. Points 1 and 4
(weight 0.5), Inset 0.2: new points at distance 0.1 instead of 0.2.

**FAL-T-04** Polygon Extrude changes topology under a falloff. Both
polygons, Shift Z 1. Without falloff: 12 points, 10 polygons (both polygons
raised to z = 1 with walls). With the falloff: 10 points, 7 polygons. Points
at x = 1 (weight 0) are not duplicated; polygon [1,2,5,4] stays in place
unchanged; polygon 0 is raised (new points (-1,0,1), (0,0,0.5), (0,1,0.5),
(-1,1,1)) with walls on its three outer sides and a wall between the raised
polygon and the unraised one. *(Why the second polygon is left out, whether
because one of its corners has weight 0 or for another reason, is not yet
established: FAL-Q04.)* Exact lists are in ORACLES.json (FAL-O-08).
