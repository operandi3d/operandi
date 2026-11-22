# Missing tools, group 1: small commands (v1, 2026-11-21)

Behaviour only. Provenance: the reference's public user guide (two short how-to pages) plus
scripted observation on the reference (2026-11-21, 24 cases), and its human-readable config
for the drop-down's choice list. REA-assisted: no. Point order after a command is NOT part of
any check below (the reference renumbers points freely); compare shapes as sets of positions.

Four tools: **Polygon Tag Type** (UI only), **Fix Black Dots**, **Edge Zip**, **Diagonal Re-Direct**.
Rows marked **PROVISIONAL** are Operandi's reading or need Jason's call: build the rest first.

## Shared inputs

- **G4:** 25 points `(i, j, 0)` for i, j = 0..4, index `5j + i`; 16 quads, quad `4j + i` =
  `[a, a+1, a+6, a+5]`, `a = 5j + i`.
- **G6:** same pattern, 6×6 quads, 49 points `(i, j, 0)`, index `7j + i`, quad `6j + i`.

---

## MT1 Polygon Tag Type (Select tab drop-down)

Operandi already has the setting (`operandi.select.polygonTagType`, Materials mode reads it).
Only the Select tab's greyed placeholder drop-down is missing.

- MT1-01 A drop-down on the Select tab, with four choices in this order: **Material**,
  **Part**, **Smoothing Group**, **Selection Set**. Default **Material**.
- MT1-02 Picking a choice runs the existing setting command; the drop-down shows the current
  value live (changing it elsewhere updates the drop-down).
- MT1-03 It only changes what Materials selection mode picks by. It does nothing in the other
  selection modes and never changes the current selection by itself.
- MT1-04 Size: tiny (UI wiring only).

---

## MT2 Fix Black Dots (Edit tab, Commands)

A "black dot" is a place where **two polygons share more than one edge in a row**: a chain of
two or more shared edges whose inner points are used by those two polygons only. Such points
shade as dark pinches on a subdivided surface.

- MT2-01 Works on the whole active mesh. The selection does not limit it (same result with or
  without a polygon selected).
- MT2-02 For each such pair, the two polygons become **one polygon**: walk the outline of the
  pair, dropping the shared chain. Its inner points are no longer used by any polygon but are
  **kept** (left as unused points, see Q-FB-1).
- MT2-03 Afterwards, Vertex mode, and the selection is the points of every fixed chain: its
  two end points and its inner points.
- MT2-04 If nothing is found: no change, message **"No black dots found."** (no undo step).
- MT2-05 One undo step.

Cases (all observed):

| Case | Input | Result |
|---|---|---|
| FB-A | Square of 2×2 split into two polygons through a middle point: points 0 (0,0,0), 1 (2,0,0), 2 (2,2,0), 3 (0,2,0), 4 (1,1,0); polygons `[0,1,2,4]`, `[0,4,2,3]` | One polygon `[0,1,2,3]`. Point 4 kept, unused. Selected points 0, 2, 4. |
| FB-B | G4 with quads 5 and 6 replaced by an extra point 25 at (2.2, 1.5, 0) and polygons `[6,7,25,12,11]`, `[7,8,13,12,25]` (they share edges 7–25 and 25–12) | Those two become one six-sided polygon `[11,6,7,8,13,12]`; point 25 kept, unused; selected 7, 12, 25. The other 14 quads unchanged. Same result with polygon 0 selected. |
| FB-C | Unit cube whose edge from (1,1,0) to (0,1,0) has an extra point 8 at (0.5,1,0) used by both faces next to it: face `[0,1,2,8,3]` and `[8,2,6,7,3]` (they share 2–8 and 8–3) | The two faces become one non-flat six-sided polygon `[0,1,2,6,7,3]`; point 8 kept, unused; selected 2, 3, 8. (Q-FB-2.) |
| FB-D | Clean G4 | "No black dots found", no change. |
| FB-E | Five triangles fanned round a centre point | No black dots. |
| FB-F | Two quads touching at one point only | No black dots. |
| FB-G | One polygon that lists a point twice `[0,1,2,2,3]` | No black dots, unchanged (Mesh Cleanup's job, not this one's). |

- **Q-FB-1 (PROVISIONAL)** The reference leaves the pinch points behind, unused. Keeping them
  matches it; removing them would be tidier. Build as the reference (keep) unless Jason says
  otherwise.
- **Q-FB-2 (PROVISIONAL)** FB-C merges two faces at a right angle into one bent polygon. That is
  what the reference does. Build it the same way for now; flagged in case Jason calls it broken.
- Size: small.

---

## MT3 Edge Zip (Edit tab, Commands)

Takes a selected edge loop (or single edges) between quads and **welds the two edge rows on
either side onto it**, removing the strip of quads on each side. It is a quick way to take a
loop out and redirect flow around poles.

- MT3-01 Edge mode, one or more edges selected. Each selected edge must have exactly **two**
  neighbouring quads.
- MT3-02 For a selected edge `a–b` with quads Q1 and Q2 on its two sides: point `a` is grouped
  with the point across Q1 from it (the Q1 neighbour of `a` that is not `b`) and the point
  across Q2 from it; the same for `b`.
- MT3-03 Groups that share a point join into one group. Every group becomes **one point at the
  average position of the group's distinct original points**. (Observed: neighbours at x = 1
  and x = 3.5, selected loop at x = 1.5: the new points sit at x = 2.0, the average of all
  three.)
- MT3-04 Q1 and Q2 collapse to nothing and are removed. Polygons that lose a point because two
  of their points joined lose the duplicate; anything left with fewer than 3 points is removed.
- MT3-05 Where a selected run **ends inside the mesh**, the two polygons just past each end each
  become a triangle. Each such pair of triangles shares the edge that continues the selected
  run; the pair is merged back into one quad (that edge removed). See EZ-B.
- MT3-06 Afterwards the selection is empty, Edge mode stays. One undo step.

Cases (observed; positions shown as (x, y), all z = 0):

| Case | Input | Result |
|---|---|---|
| EZ-A | G4, edges 2–7, 7–12, 12–17, 17–22 (the full column x = 2) | 15 points (columns x = 0, 2, 4), 8 quads, each 2 wide: per row `(0,y) (2,y) (2,y+1) (0,y+1)` and `(2,y) (4,y) (4,y+1) (2,y+1)`. |
| EZ-B | G4, edge 7–12 only | 21 points, 12 polygons, all quads: `(0,1)(2,1)(2,2)(0,2)`, `(2,1)(4,1)(4,2)(2,2)`, `(0,0)(1,0)(2,1)(0,1)`, `(3,0)(4,0)(4,1)(2,1)`, `(2,1)(1,0)(2,0)(3,0)` [merged pair, MT3-05], `(0,2)(2,2)(1,3)(0,3)`, `(2,2)(4,2)(4,3)(3,3)`, `(2,2)(3,3)(2,3)(1,3)` [merged pair], plus the 4 untouched top-row quads. |
| EZ-C | G4, edges 10–11, 11–12, 12–13, 13–14 (the full row y = 2) | 15 points (rows y = 0, 2, 4), 8 quads, each 2 tall. |
| EZ-D | Grid with uneven columns x = 0, 1, 1.5, 3.5, 4 (rows y = 0..4), full column x = 1.5 selected | Same shape as EZ-A: new column at x = 2.0. |
| EZ-E | G4, edges 6–11 and 8–13 (two edges in the same row, one quad apart) | Their groups chain together: rows y = 1 and y = 2 each collapse to one point at (2,1) and (2,2); result has two six-sided polygons `(2,1)(0,0)(1,0)(2,0)(3,0)(4,0)` and `(2,2)(4,3)(3,3)(2,3)(1,3)(0,3)` plus the 4 top-row quads. Build the same (it falls out of MT3-03). |

- **EZ-F / Q-EZ-1 (PROVISIONAL)** Open-border edge (G4, edge 0–1): the reference doesn't
  refuse; it welds (1,0) and (1,1) into (1, 0.5) and leaves two triangles. That looks like an
  accident of the script. Recommendation: refuse with "Edge Zip needs edges with a quad on each
  side". Needs Jason's OK (deviation); until then, refuse.
- Q-EZ-2 Empty selection, or edges next to non-quads: not observed. Refuse with the same
  message (Operandi's reading).
- Size: small to medium (the weld groups and the end-pair merge are the work).

---

## MT4 Diagonal Re-Direct (Edit tab, Commands)

Turns edge flow 45°: given **two quads that touch only at one corner**, it cuts both along the
diagonal through that corner and rebuilds the quads beside them so the new diagonal line is a
clean edge loop.

- MT4-01 Polygon mode. Needs **two quads sharing exactly one point** (diagonal neighbours).
  Anything else (edges selected, nothing selected, two quads sharing an edge, triangles)
  is refused: **"Select two diagonal polygons."** No change.
- MT4-02 More than two: each consecutive diagonal pair in the selection is treated (observed
  with three in a diagonal row, MT4 case DR-F). Pairs are found by shared corners.
- MT4-03 Call the shared corner **C**, the selected quads **S1, S2**, and the two quads that
  touch both of them (sharing an edge with each) the **side quads T1, T2**. The four form a 2×2
  block around C.
- MT4-04 S1 and S2 are each split into two triangles along their diagonal through C.
- MT4-05 For each side quad T that has **no point on an open border**: add one point **M** at
  T's centre (average of its four corners; only squares measured). Name T's corners C, **p1**
  (shared with S1), **F** (far from C) and **p2** (shared with S2); name the corners of S1 and
  S2 opposite C **D1** and **D2**. T and the two triangles on its side become three quads:
  `[D1, p1, M, C]`, `[C, M, p2, D2]` and `[F, p2, M, p1]`. So M joins C, p1 and p2, never F.
- MT4-06 A side quad that **does** touch an open border is left as it is, and the two triangles
  on its side stay triangles. (Observed in DR-C, DR-D, DR-E. Whether the trigger is a border
  point or a border edge isn't settled: on G4 every border-side quad has both.)
- MT4-07 Afterwards: Vertex mode, with C selected when both side quads were rebuilt; nothing
  selected when a side quad was skipped (observed). One undo step.

Cases (observed; (x, y), z = 0; only changed polygons listed):

| Case | Input | Result |
|---|---|---|
| DR-A | G4, quads 5 and 10 (C = (2,2)) | +2 points (2.5,1.5), (1.5,2.5); 18 polygons; new quads `(1,1)(2,1)(2.5,1.5)(2,2)`, `(3,1)(3,2)(2.5,1.5)(2,1)`, `(2,2)(2.5,1.5)(3,2)(3,3)`, `(1,1)(2,2)(1.5,2.5)(1,2)`, `(1,3)(1,2)(1.5,2.5)(2,3)`, `(2,2)(3,3)(2,3)(1.5,2.5)`. C selected. |
| DR-B | G4, quads 6 and 9 | Mirror image of DR-A: +(1.5,1.5), (2.5,2.5). |
| DR-C | G4, quads 1 and 6 (C = (2,1); side quad 2 is on the border, 5 is not) | +1 point (1.5,1.5); 18 polygons: triangles `(1,0)(2,0)(2,1)` and `(2,1)(3,1)(3,2)`; quads `(1,0)(2,1)(1.5,1.5)(1,1)`, `(2,1)(3,2)(2,2)(1.5,1.5)`, `(1,2)(1,1)(1.5,1.5)(2,2)`. Nothing selected. |
| DR-D | G4, quads 6 and 11 (side quad 7 on the border) | +1 point (2.5,2.5), same pattern as DR-C. |
| DR-E | G4, quads 0 and 5 (both side quads on the border) | No new points; quads 0 and 5 each become two triangles split along (0,0)–(1,1)–(2,2); 18 polygons. |
| DR-F | G4, quads 5, 10, 15 | DR-A's result, plus quad 15 split into two triangles along (3,3)–(4,4) (its side quads are on the border). |
| DR-G | G6, quads 14 and 21 (C = (3,3)) | Same pattern as DR-A, +(3.5,2.5), (2.5,3.5). |
| DR-H | Two triangles touching at a corner; two triangles sharing an edge; an edge selected; nothing selected | Refused, no change. |

- Q-DR-1 (PROVISIONAL) Non-quad side polygons, and side quads with a pole at C (more than
  four polygons round C), were not observed. Operandi's reading: refuse those pairs with the
  same message.
- Size: small to medium.
