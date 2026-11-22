# Duplicate tools v1: questions

Each has a provisional answer the builder uses until settled. DQ-1 and
DQ-4..11 are being checked through the reference (research side); Jason
only needs to answer the ones marked **Jason**.

- **DQ-1 Starting values.** DONE (probe 2026-11-08): read from the
  reference for all six tools and written into each file as CONFIRMED.
  Note Lathe's caps start ON and Mirror's Angle starts at 0.
- **DQ-2 Edge/vertex mode source.** Mirror, Clone, Array, Radial Array in
  Edge or Vertex mode copy the polygons touching the selection (DS-11).
- **DQ-3 Selection after.** Original selection kept, copies not selected
  (DS-40). **Jason**: does that match how you work, or would you rather
  the copies end up selected?
- **DQ-4 Mirror Angle.** DONE (probe): the plane turns about Left by
  Angle; 0 = plain mirror (MR-21).
- **DQ-5 Mirror first click.** The plane stays at Center (origin) rather
  than jumping to where you clicked (MR-30). **Jason**: when you mirror
  in the reference, does the plane land under your click?
- **DQ-6 Flat seam polygons.** A polygon lying in the mirror plane and its
  copy are both removed after welding (MR-43).
- **DQ-7 Lathe source.** Curves and line polygons are used up; face
  polygons stay (LT-13).
- **DQ-8 Lathe/Radial Array Axis X/Y/Z.** Read as the axis direction
  through Center, not two end points.
- **DQ-9 Lathe Count.** Segments over the whole Start→End range, and a
  full turn welds the seam (LT-21, LT-22).
- **DQ-10 Radial Array slots.** Count includes the original: CONFIRMED.
  Spacing (full turn = 360/Count, part turn ends on Start and End) still
  provisional: a scripted apply without a viewport click put the copies
  at the origin, so positions need checking by hand (RA-10).
- **DQ-11 Detriangulate Flatness.** What the percentage measures (DT-21),
  and whether the reference's 0.2 means 20 %.
- **DQ-12 Naming (Jason).** Our button says **Lathe**, with "Radial Sweep"
  in the tooltip and search. **Flip Polygons** instead of the reference's
  "Invert Polygons", as the plainer word. OK?
