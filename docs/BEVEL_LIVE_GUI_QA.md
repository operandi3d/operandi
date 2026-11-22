# Bevel live on activation: GUI checklist

Spec: `specifications/tool-activation/TOOL-ACTIVATION.md`. Fresh scene, 1 m
cube at the origin, QA app. Don't click in the viewport before the panel
edit unless the step says so.

- **BLQ-01 Polygon Bevel, Inset.** Polygon mode, select the top, press B.
  Blue and red handles show at once. Drag the Inset slider (or type 0.1,
  Return): the top insets straight away.
- **BLQ-02 Then Shift.** Type Shift 0.1: the same inset top rises; the
  status/mesh info shows 12 points, 10 polygons (one bevel, not two).
- **BLQ-03 Switch on and off.** Select the top, press B, then Drop (Q) with
  no edit: the cube is unchanged and Edit > Undo isn't "Undo Bevel".
- **BLQ-04 Second drag.** Bevel on, drag the Shift handle, release, drag it
  again: still one bevel (12 points), the second drag adds to the first.
- **BLQ-05 Drag then panel.** After a handle drag, type Inset 0.1: the same
  bevel changes.
- **BLQ-06 Shift+click.** After a bevel, Shift+click on a face: nothing
  changes yet and Inset/Shift read 0. Drag a handle or edit a value: a second
  bevel stacks on the first cap (16 points, 14 polygons). The selection
  stays as it was (provisional, TA-Q01).
- **BLQ-07 Shift+drag.** After a bevel, Shift+drag (on a handle or off the
  handles): the first bevel stays and the drag shapes a new one on its cap.
- **BLQ-08 Undo inside the tool.** Inset 0.1, Shift 0.1, then Cmd+Z: Shift
  goes back to 0, the inset stays, the tool stays on. Cmd+Z again: the cube
  is back, tool still on. Cmd+Shift+Z puts the edits back.
- **BLQ-09 Drop and Undo.** Bevel with an edit, Drop, Cmd+Z once: the cube
  is back. Save, reopen: the bevel matches what was shown.
- **BLQ-09b One step per bevel (TA-07).** Bevel, Shift+click, bevel again,
  Drop. Cmd+Z: the second bevel goes, the first stays. Cmd+Z again: the
  cube. Cmd+Shift+Z twice brings both back.
- **BLQ-10 Cancel.** Bevel on, Inset 0.2, Cancel: the cube is unchanged.
- **BLQ-11 Edge Bevel.** Edge mode, select a top edge, press B: a handle
  shows at once. Type Value 0.1: the edge bevels straight away.
- **BLQ-12 Edge Bevel, nothing selected.** Edge mode, select nothing, press
  B, type Value 0.1: every edge bevels.
- **BLQ-13 Vertex Bevel.** Vertex mode, select one corner, press B (one blue
  handle). Type Inset 0.2: the corner is cut, 1 point becomes 3.
- **BLQ-14 Extrude still waits.** Polygon mode, top selected, Shift+X: no
  handles. Type Extent Y 0.3: the mesh doesn't change until you click in the
  viewport. Smooth Shift and Thicken behave the same; Edge Chamfer waits for
  a click too.
- **BLQ-15 Polygon Bevel, nothing selected.** Polygon mode, select
  nothing, press B, type Inset 0.1: every polygon bevels at once (PB-O-01,
  kept by TOOL-ACTIVATION v1.2).
- **BLQ-16 Edge Shape and Depth.** Cube, Edge mode, one edge, B, Inset
  0.1 (Round Level 0): Depth is greyed (hover says why). Edge Shape
  Square: two strips meet at the old corner, the outline stays sharp;
  Round Level and Depth grey out. Back to Round, Round Level 1, Depth 50:
  the curve sits halfway between flat and round.
