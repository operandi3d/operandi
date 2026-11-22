# Symmetry: v2 draft questions

- **SQ-007 — Partly:** original numeric rules plus pointer pick/one-sided
  drag and both-selected/straddling Bevel/Extrude measured. Other tools open.
- **EA-Q03 — Answered for measured primitives:** Ellipsoid doubles off-plane;
  exact small Ellipsoid and Cone corner order measured in SYM-B2-04.
- **SYM-Q01 — Open/skipped:** no clean preferences, no reset performed.
- **SYM-Q02 — Answered for F1:** pointer polygon 3 adds polygon 0 (SYM-B2-10).
- **SYM-Q03 — Answered for F1:** one-sided X-handle drag leaves unselected
  counterpart unchanged; positive handle offset measured (SYM-B2-10).
- **SYM-Q04 — Partly:** fixed bounding extents do not fix tolerance; pure
  translation changes it (SYM-B2-02). Largest absolute coordinate formula
  fits samples but remains inferred outside them.
- **SYM-Q05 — Partly:** moved Work Plane Select/Move measured (SYM-B2-08);
  general transform composition at displaced/rotated planes open.
- **SYM-Q06 — Partly:** vector (1,1,0) Select/Move measured (SYM-B2-09).
- **SYM-Q07 — Partly:** one Undo for mirrored Move and symmetric Cube;
  immediate Cube Undo/Redo control exact. Intervening scripted readback
  invalidates Redo in the tested route (SYM-B2-11).
- **SYM-Q08 — Open:** hidden counterparts not run.
- **SYM-Q09 — Partly:** three displaced pairs and one isolated point tested;
  full matching ambiguities and threshold remain open (SYM-B2-06).
- **SYM-Q10 — Partly:** both sides and straddling Bevel/Extrude measured;
  Edge Bevel, Loop Slice and other complex topology remain open.
- **SYM-Q11 — Open/inconclusive:** both UV control and UV-symmetric numeric
  moves were no-ops. UV-space transform target not established.
