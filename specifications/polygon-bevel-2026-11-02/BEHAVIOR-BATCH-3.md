# Polygon Bevel — v2 draft additions

Modo 17.1v1, 2026-11-02. Numeric application, reset options then Inset 0.1,
Shift 0.2, Group Polygons on. Offset Even remains on, Edge Rail off,
Max Angle 89.9°, one segment. No symmetry, snapping or falloff.
Box is defined in README; all coordinates and polygon orders are in the
two appended oracles. REA-assisted: no.

**PB-B3-01 — Three grouped perpendicular faces meet at the intersection of the shifted planes.**
Select Box +X,+Y,+Z faces in order [3,1,4]. Result 14 points / 12 quads,
including six connecting wall quads and three selected cap quads. Original
corner 6 moves to (0.7,0.7,0.7), the intersection of X/Y/Z=0.7. All other
original points retain their positions. New points in order 8…13:
(0.7,−0.4,−0.4), (0.7,−0.4,0.7), (−0.4,−0.4,0.7),
(−0.4,0.7,−0.4), (0.7,0.7,−0.4), (−0.4,0.7,0.7).
Shared cap corners lie on two shifted planes rather than moving only
0.2 along a normalized averaged corner direction. Selection after is
[3,4,1]. Complete ordered polygons: PB-O-PB3-corner3. This answers
PB-Q04 for the requested three-face Box corner; arbitrary valence remains
unmeasured.

**PB-B3-02 — The reversed two-face pick case places +X before +Y in ascending output cap slots.**
Select original faces [3,1] in that order. Before readback confirms [3,1].
After Bevel, output cap slot 1 is +X, with corners [8,11,12,9]; slot 3
is +Y, [13,12,11,10]. Thus ascending output cap order follows this case's
pick order (3 then 1), not the original ascending face indices (1 then 3).
Output selected polygon order is [1,3]. Counts 14/12, complete points,
all faces and selection: PB-O-PB3-pick3then1. One requested case only;
this is not a universal numbering rule for every topology or selection.
Operandi may keep its own numbering by Jason's decision.
