# Mesh commands from TOOL_GAPS sections 4–6 (first pass)

**Status: v2 draft (partial); existing handed-off IDs retained.** Observed in Modo 17.1v1 on 2026-11-02 by
running each command once or twice on small fixtures. This covers the
command rows of sections 4, 5 and 6 that run without the pointer; the
interactive tools of those sections (Pen, Smooth Shift, Thicken, Spike, Poly
Haul, Sketch Extrude, Bridge beyond topic 8, Extend, Relax, Create, Merge
Tool, Tear, Vertex Slide) were not reached in this session. REA-assisted: no.

- `BEHAVIOR-POLYGON.md`: Triple, Quadruple, Spin Quad, Align, Merge,
  Collapse, Unify, Remove.
- `BEHAVIOR-EDGE.md`: Remove, Collapse, Join, Split, Subdivide.
- `BEHAVIOR-VERTEX.md`: Merge, Join, Collapse, Remove, Split, Align.
- `ORACLES.json`: 34 cases with full input, command, output and selection.

**G3** below is the 3×3 quad grid (x, y 0…3; point = row × 4 + column;
polygon k = `[a, a+1, a+5, a+4]`, a = row × 4 + column). Many commands
renumber points; the oracles give the exact lists.

## Batch 3 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), with scripted numeric cases and exact six-decimal mesh/UV readback. REA-assisted: no.

BEHAVIOR-BATCH-3.md adds 23 oracles. Covered: Count/Percent, Collapse valence 3/4/5 and border, requested Quadruple controls, Align strip and closed-loop Split.

Not covered: a positive Quadruple case, arbitrary Collapse neighborhoods, broader Align/Split rules and undo. Earlier scope text describes the prior pass; QUESTIONS.md lists remaining gaps.
