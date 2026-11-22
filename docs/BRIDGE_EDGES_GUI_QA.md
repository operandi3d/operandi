# Bridge in Edge mode: GUI checklist (2026-11-15)

For Jason or Codex, on the QA build. Spec `bridge-2026-11-05` (BR16/BR17),
questions BRI-Q05/Q07/Q08.

1. Two separate planes side by side with a gap. Edge mode, select one border
   edge on each plane facing the gap. Turn the work plane to the Bottom view
   (or any non-default one), set an action center and a falloff. Open
   Bridge: Apply is live. Apply: one quad fills the gap, shaded like its
   neighbours (not dark/inside out). The two edges stay selected.
2. Undo removes the quad in one step; Redo brings it back.
3. Select two border runs of two or three edges each (same count). Segments
   3: three rows of quads. Mode Linear / Curve / Smooth and Tension change
   the rows only when the two sides aren't flat and facing (try a floor edge
   and the bottom edge of a wall quad facing back towards it).
4. Two border loops (an open-top box and a lid with its edges): Bridge
   closes the box. Twist 1 turns the walls one corner.
5. In Edge mode, Remove Polygons, Flip Polygons and Reverse Direction are
   greyed in place with a tooltip; in Polygon mode they are live again.
6. Refusals keep Apply greyed with a reason in the panel: one edge only,
   runs of different lengths, edges inside a surface, or two sides facing
   opposite ways ("Flip one side's polygons, then bridge").
