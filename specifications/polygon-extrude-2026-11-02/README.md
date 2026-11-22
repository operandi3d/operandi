# Polygon Extrude: modes, odd input and non-manifold input

**Status: partial.** Observed in Modo 17.1v1 on 2026-11-02 through scripted
probes (numeric application). Scope: the Extrude row of TOOL_GAPS section 4
(EX-Q01, EX-Q04 parts). REA-assisted: no.

## Covered

- `BEHAVIOR-MODES.md`: defaults; Auto, Thick and Shift modes; what is built
  and selected.
- `BEHAVIOR-ODD-INPUT.md`: zero and negative shift, sideways shift, empty
  selection, non-planar polygons, segments.
- `BEHAVIOR-NONMANIFOLD.md`: polygons sharing an edge with two others,
  polygons touching at one point, two polygons facing different ways, a
  transformed item.
- `ORACLES.json`: all 21 cases, full point/polygon lists and selection.

## Not covered (QUESTIONS.md)

UV option (needs a selected texture map; not exercised), weights/colors
(EX-Q03), physical hauling (EX-Q02), Tool States (EX-Q05), subdivision
(EX-Q06), undo.

## Fixtures

- **Quad**: (0,0,0), (1,0,0), (1,1,0), (0,1,0); polygon `[0,1,2,3]`
  (normal +Z).
- **Two**: points (0,0,0), (1,0,0), (2,0,0), (0,1,0), (1,1,0), (2,1,0);
  polygons `[0,1,4,3]`, `[1,2,5,4]`.
- **Fan**: points (0,0,0), (0,1,0), (1,0,0), (1,1,0), (-1,0,0), (-1,1,0),
  (0,0,1), (0,1,1); polygons `[0,2,3,1]`, `[4,0,1,5]`, `[0,1,7,6]`: three
  quads sharing edge 0–1.
- **Bowtie**: (0,0,0), (1,0,0), (1,1,0), (0,1,0), (2,1,0), (2,2,0),
  (1,2,0); polygons `[0,1,2,3]`, `[2,4,5,6]` touching at point 2.
- **Cross**: Quad plus a second quad `[4,5,6,7]` at x = 3 in the YZ plane:
  (3,0,0), (3,0,1), (3,1,1), (3,1,0).
