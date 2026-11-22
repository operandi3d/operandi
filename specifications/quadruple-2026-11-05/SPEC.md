# Quadruple (Convert to Quadrangles) — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Quadruple (Geometry > Polygon > Quadruple; "Convert to Quadrangles" in the
Topology layout) has no options. It splits each selected n-gon into a strip
of quads by adding straight edges ("rungs") between opposite corners.
Points never move and no points are added.

## Rules

1. **Rungs:** for an n-gon with corners c0…c(n−1) (n even), the rungs join
   c(i) and c(n−1−i) for i = 1 … n/2−1, counted from one end of the strip.
   An 8-gon gives 3 quads (QD01), a 6-gon 2 (QD02), a 12-gon 5 (QD03).
2. **Where the strip starts doesn't depend on the corner list's starting
   point:** the same 8-gon listed from corner 2 or corner 4 gives the same
   quads (QD04, QD05). Modo picks the two strip ends itself; where edge
   lengths differ, the ends were the short edges (QD17), except QD18
   (rule 7).
3. **Odd corner count:** the last piece is a triangle: a 9-gon (5 + 4)
   gives 3 quads and a triangle (QD06), a pentagon a quad and a triangle
   (QD09).
4. **Unequal rows** (6 + 4) still give quads only, pairing as in rule 1
   (QD07). Non-planar and regular (octagon) n-gons are split the same way
   (QD08, QD11).
5. **Quads and triangles are left alone** (QD10).
6. **Which polygons:** the selected polygons (QD12, QD13); polygon mode
   with nothing selected → every polygon (QD14). Vertex mode is refused with
   "Need to directly select one or more polygons." (QD16). Neighbouring
   polygons are unchanged (QD15).
7. **Long thin strips** are split across the short way (QD17). On a tall
   strip whose rows are much shorter than its ends, Modo instead joined
   points of the same row, giving a degenerate quad (QD18). **Decided for
   Operandi (Q-QD-01):** pick the ends with the shortest total new edges,
   never joining points of the same row; QD18 is a Modo bug, not copied.
8. **Result order and selection:** with one n-gon, its first quad keeps
   the n-gon's polygon index and the rest are appended (QD15); with
   several, the order is Modo's own (Q-QD-03). If the n-gons were selected,
   all new pieces are selected afterwards; with nothing selected, nothing
   is (QD14).

## Open questions and decisions

- **Q-QD-01** (decided below) How the strip ends are chosen. In QD18 (rows much
  shorter than the ends) Modo made a degenerate quad from points of one
  row. Sane rule for Operandi: take the two strip ends as the pair of
  opposite edges that gives the shortest total rung length while never
  joining points of the same row. That matches every case except QD18,
  which counts as a Modo bug.
- **Q-QD-02** Tie-breaking when several start choices give equal rungs
  (QD01, QD08).
- **Q-QD-03** Polygon order when several n-gons are converted at once
  (QD12).

## Decided (2026-11-05)

- **Q-QD-01 → decided:** choose the two strip ends that give the shortest
  total new edges while never joining points of the same row. QD18's
  degenerate result is a Modo bug and is not copied.
