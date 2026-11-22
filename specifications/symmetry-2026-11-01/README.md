# Symmetry (shared system)

**Status: v2 draft (partial); v1 already handed off.** Observed in Modo 17.1v1 on 2026-11-01 through scripted
probes (commands run inside Modo; results read back from the mesh). Nothing
here is taken from the user guide except tool and option names.
REA-assisted: no.

## Covered

- `BEHAVIOR-STATE.md`: the Symmetry setting, its axis, offset and the plane's
  space.
- `BEHAVIOR-TRANSFORMS.md`: how Move, Rotate and Scale treat selected
  elements on each side of the plane.
- `BEHAVIOR-SELECT-SYMMETRICAL.md`: adding counterparts to a selection;
  matching tolerance; the Topology option.
- `BEHAVIOR-PRIMITIVES.md`: primitives created while Symmetry is on.
- `BEHAVIOR-SYMMETRIZE.md`: making a mesh exactly symmetric.
- `ORACLES.json`: measured cases (inputs are small meshes defined inline).

## Not covered yet (see QUESTIONS.md)

- What a **pointer** pick or a **viewport drag** does with Symmetry on. All
  observations here are numeric (scripted) applications. In that mode an
  unselected counterpart is never edited; whether a pointer click also picks
  the counterpart could not be observed without viewport input.
- Topology tools (Bevel, Extrude, Edge Bevel, Loop Slice…) beyond one Bevel
  case, the Work Plane and Arbitrary-axis options, the centerline selection
  set, the Symmetry Tool, UV symmetry.
- Undo/redo: undo cannot be driven from a script in Modo, so step counts are
  unobserved.
- Defaults on a fresh install (the session's values are recorded, but they may
  carry Jason's preferences).

## Fixture used throughout

**F1**: one mesh item at identity, ten points and four quads in a strip:
points 0–4 at y = 0 and points 5–9 at y = 1, both rows at x = -2, -1, 0, 1, 2,
z = 0. Polygon *k* (k = 0…3) is `[k, k+1, k+6, k+5]`. Point 7 = (0,1,0) lies
on the X plane; points 5 and 9 are a mirrored pair.

## Batch 2 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex) and scripted probes; results read back from Modo by script (six decimals). REA-assisted: no.

BEHAVIOR-BATCH-2.md adds 93 oracles, preserving every v1 oracle. Covered: ordered selection, translation-sensitive tolerance, plane Rotate/Scale, exact primitive winding and center-plane exception, pairing/topology, moved Work Plane/Arbitrary examples, pointer picking/drag, topology edits and menu undo/redo control. UV symmetry remains inconclusive. Defaults skipped without resetting preferences. The older Not covered list describes v1; current dispositions are in QUESTIONS.md.
