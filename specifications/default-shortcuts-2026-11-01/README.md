# Default keyboard shortcuts (macOS)

**Status: v2 draft (partial); earlier handed-off IDs retained.**

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), results
read back from Modo (six-decimal mesh coordinates) and UI.
REA-assisted: no.

Key map read from the default configuration shipped with Modo 17.1v1,
23 bindings verified by key press. The raw inventory retains 1,233 entries
from 16 shipped input-remapping sections, including pointer/editor bindings.
It is not an export of the current user configuration; no defaults were reset.

## Covered

- BEHAVIOR-KEYS.md: measured unmodified tool, mode and selection keys;
  B and S across Vertex, Edge and Polygon modes; configuration candidates.
- ORACLES.json: 23 cases with exact fixture and before/after results.
- 32 raw key cases, including eight unresolved modifier attempts and one
  tool whose UI name needs confirmation. Input Editor confirms Shift+C's
  Polygon Slice assignment and Shift+B's Polygon Bevel assignment.

## Not covered

Physical Shift+C, Option+C, Control+E and Command+E are now observed in
one Polygon context; the other modified combinations remain unverified.
Broader Command/Control mapping remains open. The full modeling-tool inventory,
all mode-dependent keys, hold/release actions, undo/redo and primitive,
falloff, symmetry and view keys need further verification.


## Physical-input additions

Four physical cases observed on 2026-11-03 UTC (2026-11-02 local), Jason at
the keyboard and Codex reading native UI plus six-decimal mesh output.
BEHAVIOR-PHYSICAL-KEYS.md adds four oracles (27 total): Shift+C → Polygon
Slice, Option+C → Loop Slice, Control+E → Bend, Command+E → no visible
activation observed. Loop Slice readback includes active preview topology.
All 23 previous oracle objects are retained unchanged. The earlier coverage
and binding-count paragraph describes the first desktop pass. No current
configuration reset or implementation hand-off performed.
