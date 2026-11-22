# Falloff (shared system)

**Status: v3 draft (partial); existing handed-off IDs retained.** Observed in Modo 17.1v1 on 2026-11-01 through scripted
probes. REA-assisted: no.

## Covered

- `BEHAVIOR-MECHANISM.md`: one falloff at a time, attribute retention,
  Auto Size and Reverse.
- `BEHAVIOR-LINEAR.md`: Linear falloff weights, all shapes, Symmetric,
  points off the line.
- `BEHAVIOR-RADIAL.md`: Radial falloff weights and non-uniform size.
- `BEHAVIOR-TOOLS.md`: what Bevel, Extrude, Edge Bevel and Vertex Bevel do
  under a falloff.
- `ORACLES.json`: measured weights and tool results.

Weights were measured by applying a numeric Move of exactly 1 m with the
falloff active: the displacement of each point equals its weight.

## Not covered (see QUESTIONS.md)

Selection, Soft Selection and Element falloffs (they did not act on a
numerically applied transform, so they need pointer input to observe); the
other falloff types (Airbrush, Coplanar, Curvature, Cylinder, Image, Lasso,
Noise, Path, Screen, Vertex Map); Mix modes; Use World Transforms; undo.

## Batch 2 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex) and scripted probes; results read back from Modo by script (six decimals). REA-assisted: no. BEHAVIOR-BATCH-2.md adds 19 oracles without changing earlier oracle meanings. Covers zero/small-positive Extrude, weighted Rotate/Scale, symmetry weight sampling, zero radial axis, UI In/Out, translated-item world option and menu Undo. The older Not covered list describes v1. Smooth Shift crashed at 21:19:51 UTC during UI discovery and was skipped; Thicken and pointer falloff cases remain blocked by coordinate input after relaunch.

## Batch 3 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), with scripted numeric cases and exact six-decimal mesh/UV readback. REA-assisted: no.

BEHAVIOR-BATCH-3.md adds 26 oracles. Covered: seven additional tool/control pairs, zero-weight topology and averaged Spike weights; single-edge Slide discriminators.

Not covered: other modes, pointer operations, general Bridge rule, and the previously listed unmeasured falloff types/options. Earlier scope text describes the prior pass; QUESTIONS.md lists remaining gaps.
