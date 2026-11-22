# UVs on points created by edge, vertex and fill tools (SQ-UV-01)

**Status: v2 draft (partial); prior version already handed off.** Observed in Modo
17.1v1 on 2026-11-02 through scripted probes. REA-assisted: no.

## Covered

Edge Bevel, Edge Chamfer, Edge Extrude, Vertex Bevel, Vertex Extrude, Grow
Quads, Fill Quads, Bridge and Chamfer Edit, each run once on a mesh whose UV
map is continuous and once with a UV seam next to the edit.

- `BEHAVIOR-UV-RULES.md`: the rules seen across tools.
- `BEHAVIOR-PER-TOOL.md`: what each tool did, including seams, odd results
  and selection afterwards.
- `ORACLES.json`: all 18 cases with full input and output (points, polygons
  in order, UV of every polygon corner, selection).

## Fixtures

- **G3**: 3×3 quads in the XY plane, x and y from 0 to 3 in 1 m steps.
  Point index = row × 4 + column; polygon index = row × 3 + column; polygon
  *k* in row *r*, column *c* is `[a, a+1, a+5, a+4]` with a = r × 4 + c.
  UV map "Texture": every point has the continuous value (x/3, y/3).
- **G3 with seam**: G3, plus polygons 2, 5 and 8 (the right column, x 2…3)
  carry their own corner values (x/3 + 1, y/3). The line x = 2 is then a
  seam: points on it have one UV in the left polygons and another in the
  right ones.
- **G4 hole** (Fill Quads): 4×4 quads, x and y 0…4, with the middle four
  polygons removed (12 polygons remain, in the original order). Same UV rule;
  in the seam variant the polygons at x 3…4 carry (x/3 + 1, y/3).
- **Two quads** (Bridge): `[0,1,2,3]` at x 0…1 and `[4,5,6,7]` at x 2…3,
  y 0…1. Seam variant: the second quad carries (x/3 + 1, y/3).

In the text, "the map value at a point" means (x/3, y/3) for left-side
polygons and (x/3 + 1, y/3) for seam polygons.

## Not covered (see QUESTIONS.md)

Corner weights, vertex colors, normals and other typed maps: no reliable way
to set them up as test input yet, so they are unobserved. Non-planar input, transformed items, undo/redo.

## Batch 2 additions

Observed in Modo 17.1v1 on 2026-11-02 by scripted probes (Codex), results read back from Modo by script (six decimals). REA-assisted: no. BEHAVIOR-BATCH-2.md adds 8 oracles without changing prior IDs or values. Covers non-affine Edge Bevel edge interpolation, left-seam frame choice, and Profile/Triangle Fan with Stop At Edge on/off. Other corner map types, curved interior interpolation, general frame choice and undo remain open.
