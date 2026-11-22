# Polygon Bevel: v2 draft questions

Answered here:

- **M17-Q02 / Q18-03 (part)**: grouped Bevel of adjoining faces at 90° and
  45°: shared-edge points sit where the moved face planes meet (PB-G-01,
  PB-G-03); Offset Even changes border-corner insets (PB-G-04).
- **M17-Q04**: Modo does not clamp or refuse a collapsing inset; Auto Weld
  merges the collapsed corners and removes the inner polygon (PB-C-01…04).
  Operandi refuses these by Jason's choice; this records what the reference
  does.

Still open:

- **BV-Q01**: dedicated polygon-weight providers.
- **BV-Q02**: arbitrary adaptive-profile sampling.
- **BV-Q04**: sparse or conflicting maps (weight maps could not be authored
  by script without crashing Modo; see the new-point-uvs topic).
- PB-Q01: Use Vertex Normal Map on/off with a custom normal map.
- PB-Q02: UVs on bevel walls for each UVs option.
- PB-Q03: Max Angle and Edge Rail on curved/smoothed regions (no effect on
  the box).
- PB-Q04 — Answered for the requested Box +X/+Y/+Z corner (PB-B3-01);
  arbitrary valence remains open.
- PB-Q05: undo/redo (not drivable by script).

- PB-Q06 — Answered for the one requested pick [3,1] case: output cap slots follow +X then +Y, matching pick order. Broader numbering is not specified; Operandi keeps its own numbering (PB-B3-02).
