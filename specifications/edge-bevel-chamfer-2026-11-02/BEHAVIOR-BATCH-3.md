# Edge Bevel / Edge Chamfer — v2 draft additions

Modo 17.1v1, 2026-11-02, scripted numeric operations observed by Codex;
six-decimal point/UV readback. REA-assisted: no. Reset each tool, then apply
the stated overrides. No symmetry, snapping or falloff. Box and corner3 are
defined in README. Every oracle contains every output point and polygon.

**EBC-B3-01 — Edge Bevel Width produces an angle-dependent strip in the tested regular prisms.**
Radius-one prism/tube, Y=−0.5…0.5, one longitudinal edge at (X,Z)=(1,0).
Mode Width, Width 0.2, Round Level zero. Measure perpendicular to the old
longitudinal edge in the lower-end cross-section:

| Interior angle | Sides | Distance from old edge to each cut edge | Distance between cut edges |
|---|---|---|---|
| 60° | 3, capped prism | 0.200000 | 0.200000 |
| 90° | 4, open tube | 0.282843 | 0.400000 |
| 120° | 6, open tube | 0.346410 | 0.600000 |
| 135° | 8, open tube | 0.369551 | 0.682842 |

Distances are derived from the recorded six-decimal points. The samples fit
side distance = 2×Width×sin(interior-angle/2), within 1e-5; neither suggested
half-width divided by sine/cosine rule fits these four samples. This is a
fit for the tested regular fixtures, not a universal formula for every
junction. EBC-O-E3-01-sides3/sides8 and EBC-O-E3-01-bevel-sides4/sides6.
Additional Width 0.2 Chamfer controls give side/strip distances
0.136819/0.136818 on the capped triangle and 0.184776/0.341422 on the open
octagon; their general angle/end-topology dependence remains open
(EBC-O-E3-01-chamfer-sides3/sides8).

**EBC-B3-02 — Tested Round Levels 1…4 give twice the level in profile segments.**
Box edge 6–7, Inset 0.1, Round. Level 1 → 2 segments, 12 points / 8
polygons; Level 3 → 6 segments, 20/12; Level 4 → 8 segments, 24/14.
Earlier Level 2 gives 4 segments; Level zero gives one. Thus the prior
2^Level interpretation in EBC-P-01 is superseded; the original Level 0/2
measurements remain valid. At X=±0.5 every profile sample lies on the
quarter-circle in Y/Z centered (0.4,0.4), radius 0.1. Level 1 has endpoints
(0.4,0.5)/(0.5,0.4) and middle (0.470711,0.470711); Levels 3/4 sample
every 15°/11.25°. Every profile point is in EBC-O-E3-02-level1/level3/level4.
Higher levels beyond 4 remain unmeasured.

**EBC-B3-03 — The two-edge top-corner displacement matches adding an Inset-length vector along each selected edge.**
Capped radius-one prisms, Y=±0.5. Select the two top edges meeting at
(1,0.5,0), Inset 0.1. Hexagon (120°): the common top-face point becomes
(0.9,0.5,0). Triangle (60°): (0.826795,0.5,0). These are the sum of the
two inward unit edge directions multiplied by 0.1: bisector distances
0.100000 and 0.173205. They are not the intersection of lines each
parallel-offset by 0.1 on the top face; the measured perpendicular distance
to each original top edge is about 0.086603 in both cases. Counts 15/10
and 9/7. EBC-O-E3-03-sides6/sides3.

**EBC-B3-04 — Multi-row corner3 patches have the following measured face layouts.**
Box corner3, Inset/Offset 0.1. The corner patch is the faces wholly inside
the near-corner cube X,Y,Z>0.35; the full mesh remains in each oracle:

| Tool / profile count | Whole mesh points / polygons | Corner patch |
|---|---|---|
| Chamfer 3 segments | 28 / 22 | 6 quads around one central triangle |
| Chamfer 4 segments | 38 / 30 | 12 quads, one shared center point |
| Bevel Round Level 3 (6 segments) | 62 / 51 | 27 quads, one shared center point |
| Chamfer 6, extra control | 62 / 51 | 27 quads, same geometric point set as Bevel Level 3 |

The even-profile central point is (0.460948,0.460948,0.460948), also found
in the earlier 2-segment case. No single center point occurs with three
segments. Point numbering/selection may differ between tools. Exact patch
faces and every point: EBC-O-E3-04-chamfer-seg3/seg4/seg6-control and
EBC-O-E3-04-bevel-level3. This is a measured patch classification, not a
general meshing algorithm beyond these cases.

**EBC-B3-05 — New center UVs in the tested continuous map differ from evaluating the displaced center's coordinates.**
Assign every original Box point continuous Texture UV=(X/3,Y/3). Chamfer
segments 2 and 4 and Bevel Round Level 3 keep the geometric center above,
but every polygon corner at that center has UV (0.144444,0.144444),
rather than (0.153649,0.153649) from its displaced coordinates. Chamfer 3
has central triangle positions and corresponding UVs:

| Point position | UV |
|---|---|
| (0.449285,0.476583,0.449285) | (0.142623,0.148087) |
| (0.449285,0.449285,0.476583) | (0.142623,0.142623) |
| (0.476583,0.449285,0.449285) | (0.148087,0.142623) |

The map does not change geometry versus the corresponding no-map controls.
Every new point's per-corner UV is recorded, without extrapolating an
unmeasured UV rule: EBC-O-E3-05-chamfer-seg2/seg3/seg4 and
EBC-O-E3-05-bevel-level3.

**EBC-B3-06 — A selected edge may terminate directly on the mesh border without an added outside cap.**
G3, edge 1–5 from (1,0,0) to (1,1,0), Inset/Offset 0.1. Bevel gives
19/11: strip sides at X=0.9/1.1 end on Y=0; the interior end has a
triangle through (0.9,1,0),(1,1.1,0),(1.1,1,0). Chamfer (one segment,
Profile) gives 19/12 with two coincident zero-area interior end triangles.
The border end remains part of the open boundary. Select the whole row
4–5–6–7 instead: both tools give 20/12, three strip quads spanning X=0…3
between Y=0.9 and 1.1; no extra cap extends past either border. Exact
polygons/selection: EBC-O-E3-06-one-bevel/chamfer and
EBC-O-E3-06-row-bevel/chamfer.

**EBC-B3-07 — Edge End changes the planar T-junction result.**
G3 T edges 4–5,5–6,5–9; Chamfer Offset 0.1, Segments 3. Profile →
38 points / 33 polygons (20 quads, 7 triangles, 6 pentagons), including
eight zero-area polygons at the ends. Triangle Fan → 38/31 (20 quads,
7 triangles, 4 pentagons), with no zero-area polygons. These are not
equivalent outputs in this fixture. All point positions, corner order and
selection are in EBC-O-E3-07-profile/triangleFan.

**EBC-B3-08 — Square and Sharp retain the same three-point profile across the tested Levels 1…3.**
Box edge 6–7, Inset 0.1. For both shapes and each level, counts are 12/8.
At each end the path is (Y,Z)=(0.4,0.5)→(0.5,0.5)→(0.5,0.4): two flat
segments through the original corner. Level does not add more samples in
these six cases. EBC-O-E3-08-square-level1/level2/level3 and
EBC-O-E3-08-sharp-level1/level2/level3. Other shape controls remain open.
