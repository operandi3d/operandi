# Bend v2 — GUI check (about 3 minutes)

QA build from branch `bend-v2` (spec bend-handles-2026-11-15).

1. New scene, Capsule standing up Y. Polygon mode, nothing selected.
2. Ctrl+E (or Deform tab ▸ Bend): a blue ring at the capsule's middle, a
   thin violet line up to a small cyan cross; nothing moves. Hover the ring
   and the cross: each lights up and shows its tooltip.
3. Drag round the ring: the capsule curls; Angle updates. Hold Ctrl at the
   press: Angle goes in steps (BH-Q06).
4. Axis row X / Y / Z: the ring turns at once and the bend curls the other
   way; centre and spine stay.
5. Drag the cross: the spine changes; Ctrl keeps it on one axis. With
   snapping on, it snaps to a point (BH-Q07).
6. Click on the capsule near its bottom: the ring and the bend start there.
   Does the cross stay with the spine vector (BH-Q05)? Click and keep
   dragging: only the centre moves (BH-Q08). With snapping on, a click
   near a point lands the centre on it.
7. Space drops; Undo/Redo; the Action Center menu is back to what it was.
