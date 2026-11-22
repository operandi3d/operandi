# Make (P) with edges selected — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Rules

1. **A closed loop of border edges** makes one new polygon over the hole
   (A3: lidless box, whole rim selected → a lid on the rim's 4 existing
   points, facing outward (+Y), 8 points / 6 polygons). The new polygon is
   selected; Edge mode stays.
2. **Two bare edges** (2-point line polygons, no faces) make nothing (A1).
3. **Two border edges of separate quads facing across a gap** made nothing
   either (A2), though the guide says two edges make a quad. Open
   (Q-ME-01).

## Open questions and decisions

- **Q-ME-01** The guide's "select two edges and press P" case: which edge
  pairs qualify (A1 and A2 made nothing). Try two border edges of the same
  open mesh, and edges sharing a point.
