# Edge Bevel and Edge Chamfer: junctions, shared points, profiles, modes

**Status: v2 draft (partial); existing handed-off IDs retained.** Observed in Modo 17.1v1 on 2026-11-02 through scripted
probes (numeric application). REA-assisted: no.

## Covered

- `BEHAVIOR-DEFAULTS.md`: values after Reset and accepted option values.
- `BEHAVIOR-JUNCTIONS.md`: one edge, two edges sharing a point, three edges
  at a corner, a closed loop; selection after.
- `BEHAVIOR-PROFILES.md`: Round Level / Segments, Shape round/square,
  Depth, and Inset vs Width mode.
- `ORACLES.json`: 21 cases with full point and polygon lists.

## Fixture

**Box**: points 0 (-0.5,-0.5,-0.5), 1 (0.5,-0.5,-0.5), 2 (0.5,-0.5,0.5),
3 (-0.5,-0.5,0.5), 4 (-0.5,0.5,-0.5), 5 (0.5,0.5,-0.5), 6 (0.5,0.5,0.5),
7 (-0.5,0.5,0.5); faces `[0,1,2,3]` (-Y), `[7,6,5,4]` (+Y), `[0,4,5,1]`
(-Z), `[1,5,6,2]` (+X), `[2,6,7,3]` (+Z), `[3,7,4,0]` (-X).

Edge sets: **one** = 6–7; **L** = 6–7 and 5–6 (sharing point 6);
**corner3** = 6–7, 5–6, 2–6 (all three edges at corner 6); **toploop** =
4–5, 5–6, 6–7, 7–4.

## Not covered (QUESTIONS.md)

Interior edges of flat grids beyond the topic-8 cases, Mitering, Coplanar,
Sharp, Parallel, Flatness, Boundary Extend, Material override, UVs (see
topic 8), undo.

## Batch 3 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), with scripted numeric cases and exact six-decimal mesh/UV readback. REA-assisted: no.

BEHAVIOR-BATCH-3.md adds 31 oracles. Covered: non-90° Width/miter, Round Levels 1/3/4, 3/4/6-row patches and UVs, open borders, planar T ends and Square/Sharp levels.

Not covered: broader Width/junction laws, remaining mode/material options, arbitrary UV maps and undo. Earlier scope text describes the prior pass; QUESTIONS.md lists remaining gaps.
