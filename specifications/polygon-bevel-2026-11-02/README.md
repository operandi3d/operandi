# Polygon Bevel: grouped non-coplanar faces, collapsing insets, odd input

**Status: v2 draft (partial); existing handed-off IDs retained.** Observed in Modo 17.1v1 on 2026-11-02 through scripted
probes (numeric application of the tool). Scope is the Bevel row of
TOOL_GAPS section 4, beyond what Operandi already has. REA-assisted: no.

## Covered

- `BEHAVIOR-DEFAULTS.md`: every option's value after Reset.
- `BEHAVIOR-GROUPED.md`: Group Polygons on adjoining faces at 90° and at
  45° (M17-Q02, Q18-03), with Offset Even, Edge Rail and Max Angle.
- `BEHAVIOR-INSET-COLLAPSE.md`: insets larger than the polygon, Auto Weld,
  concave polygons (M17-Q04).
- `BEHAVIOR-ODD-INPUT.md`: empty selection, negative and zero values.
- `ORACLES.json`: all 20 cases with full point and polygon lists.

## Not covered (QUESTIONS.md)

BV-Q01 polygon-weight providers, BV-Q02 adaptive profiles, BV-Q04 sparse or
conflicting maps, the vertex-normal-map option, UVs on bevel walls, undo.

## Fixtures

- **Box**: points (-0.5,-0.5,-0.5), (0.5,-0.5,-0.5), (0.5,-0.5,0.5),
  (-0.5,-0.5,0.5), (-0.5,0.5,-0.5), (0.5,0.5,-0.5), (0.5,0.5,0.5),
  (-0.5,0.5,0.5); faces 0 `[0,1,2,3]` (-Y), 1 `[7,6,5,4]` (+Y),
  2 `[0,4,5,1]` (-Z), 3 `[1,5,6,2]` (+X), 4 `[2,6,7,3]` (+Z),
  5 `[3,7,4,0]` (-X).
- **Cyl8**: open 8-sided tube, radius 1, y -0.5…0.5; points 0–7 at y = -0.5
  and 8–15 at y = 0.5, point i at angle 45°·i in XZ; side polygon i is
  `[i, i+8, (i+1)%8+8, (i+1)%8]`.
- **Square** (0,0)–(1,1), **Rect** (0,0)–(2,1), **L** with points (0,0),
  (2,0), (2,1), (1,1), (1,2), (0,2): single polygons in the XY plane.

## Batch 3 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), with scripted numeric cases and exact six-decimal mesh/UV readback. REA-assisted: no.

BEHAVIOR-BATCH-3.md adds 2 oracles. Covered: grouped +X/+Y/+Z Box corner and the single reversed two-face selection-order case.

Not covered: arbitrary grouped valence, remaining map/UV/normal controls, adaptive profiles and undo. Earlier scope text describes the prior pass; QUESTIONS.md lists remaining gaps.
