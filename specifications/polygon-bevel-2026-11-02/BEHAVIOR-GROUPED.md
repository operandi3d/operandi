# Grouped bevel of adjoining non-coplanar faces

**PB-G-01 Group on, two faces at 90°.** Box, faces 1 (+Y) and 3 (+X), Shift
0.2, Inset 0.1. 8 → 14 points, 6 → 12 polygons. The two faces are beveled as
one region: their shared edge is not walled.
- Points on the shared edge, (0.5,0.5,∓0.5), move to (0.7, 0.7, ∓0.4): each
  face's plane moves out by exactly Shift, and the point sits where the two
  moved planes meet; the inset applies only along the region border (z).
- The other border corners move out along their own face by Shift and in
  by Inset along both border edges: e.g. (0.5,-0.5,-0.5) → (0.7,-0.4,-0.4).
- Polygon order: the selected faces keep their indices with new corners
  (face 1 → `[13,12,11,10]`, face 3 → `[8,11,12,9]`); six wall quads are
  appended (indices 6–11). Selection after: faces 3 and 1.

**PB-G-02 Group off** bevels each face separately: 16 points, 14 polygons;
the top face goes to y = 0.7 with corners inset 0.1 on all four sides, the
+X face likewise, and walls are built along the shared edge too.

**PB-G-03 Two faces at 45° (Cyl8 faces 0 and 1), Group on, Shift 0.2,
Inset 0.1.** The shared-edge points move along the bisector of the two face
normals by Shift / cos(22.5°) = 0.216478, so each face plane moves out by
exactly 0.2: point 1 (0.707107, ∓0.5, 0.707107) → (0.86018, ∓0.4, 0.86018).
16 → 22 points, 8 → 14 polygons.

**PB-G-04 Offset Even off** changes only the inset at border corners: the
corner moves by Inset along the corner's bisector instead of by Inset from
each edge. Box faces 1 + 3, Shift 0.2, Inset 0.1: (0.5,-0.5,-0.5) →
(0.7, -0.429289, -0.429289) (each coordinate moves 0.1/√2) instead of
(0.7,-0.4,-0.4). Shared-edge points are unchanged ((0.7,0.7,∓0.4)).

**PB-G-05** Max Angle 30° and Edge Rail on gave results identical to the
defaults in PB-G-01. *(Their effect on curved or smoothed input is open:
PB-Q03.)*

Shift only and Inset only variants of PB-G-01 are in ORACLES.json.
