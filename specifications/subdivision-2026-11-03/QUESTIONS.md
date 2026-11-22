# Subdivision: open questions

- **SDV-Q01 — Answered (v2)**: see SDV-04, SDV-09, SDV-10.
- **SDV-Q02**: How edge weights act on Subdivision polygons. Nine points move
  at level 1. Neither semi-sharp creases in the Catmull-Clark step nor any
  change to that step's points alone reproduces the result, so the weight
  also enters the patch stage. A candidate for an REA question if needed.
- **SDV-Q03**: How Crease Edges differs from Smooth (the guide says only with
  edge weighting); test an open grid with a weighted boundary edge.
- **SDV-Q04**: Vertex weights (point values of the Subdivision map) for both
  kinds.
- **SDV-Q05**: Tab while a viewport value field is open, and Tab in other
  panels.
- **SDV-Q06 — Partly (v3)**: Freeze keeps UV maps (SDV-15). Still open: UV
  smoothing on multi-quad islands, Linear UVs, UV Boundary Rule, weights
  after Freeze.
- **SDV-Q07 — Partly (v3)**: Face + smooth answered (SDV-13). Still open: a
  mesh mixing Subdivision and Catmull-Clark (SDV-14).
- **SDV-Q08**: Maximum levels, render levels, adaptive subdivision, and the
  display options (cage drawing, OpenSubdiv draw).
- **SDV-Q09**: Undo of a toggle and of a level change.
