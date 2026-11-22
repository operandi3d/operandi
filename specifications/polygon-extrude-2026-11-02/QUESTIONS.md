# Polygon Extrude: open questions

Answered here:

- **EX-Q01 (part)**: non-planar polygons (common shift vector, no refusal),
  edge shared by three polygons, polygons touching at a point, polygons
  facing different ways (BEHAVIOR-ODD-INPUT, BEHAVIOR-NONMANIFOLD).
- **EX-Q04 (part)**: the shift vector is in world space on a rotated item.

Still open:

- PE-Q01: confirm the Auto rule (Thick for isolated regions, Shift for
  regions with unselected neighbours) on a grid with a block selected.
- **EX-Q02**: hauling gain and modifiers (pointer input).
- **EX-Q03**: corner-only weight/color storage (could not author weights by
  script without crashing Modo).
- **EX-Q05**: Tool States UI.
- **EX-Q06**: evaluated subdivision.
- PE-Q02: the UVs option (U/V/None) needs a selected texture map in the
  tool; setting it failed without one ("only applies when a texture map has
  been selected").
- PE-Q03: undo/redo (not drivable by script).
