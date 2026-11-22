# Click to start: GUI check (branch click-start)

For Jason, at the mouse, about 5 minutes. Use the QA app named in the
hand-back. Front view unless a line says otherwise.

1. **Bend on a lying capsule.** Add a Capsule, lay it along Z (Rotate X
   90°). Look from the front. Pick Bend: no ring, no spine, the mesh
   doesn't move, and the status bar says "Click in a view to start Bend,
   or set values and press Apply."
2. Click on the capsule's face. The ring appears where you clicked, on
   the surface, not behind it. The spine runs along X or Y (never toward
   you along Z), to the far end of the capsule. Axis shows Z.
3. Drag the ring: it bends. Click somewhere else on the capsule: the
   centre jumps there and the spine refits. Axis goes back to Z even if
   you'd picked X.
4. **CS-Q02:** with snapping off, does the click land on the surface (as
   built) or on the work plane?
5. **Armed Apply (CS-21, CS-Q03).** Pick Bend, type Angle 90 in Tool
   Properties, press Apply. The capsule bends once, nothing is drawn, and
   the tool still waits for a click. Undo removes it in one step.
6. **Drop when armed (CS-22).** Pick Bend, then Drop: nothing changes and
   History has no new step.
7. **Bevels (CS-Q01, approved deviation).** Select polygons, pick Bevel:
   no handles until you press in the view. Press and drag: the bevel
   follows the drag as before. Drag again: it edits the same bevel. Same
   for Edge Bevel, Vertex Bevel and Edge Chamfer. Vertex Extrude starts
   at once.
8. **CS-Q04.** Press and keep dragging on the first click with Taper and
   with Push: is the drag the tool's normal drag?
9. Move, Rotate, Scale, Twist and Loop Slice still start at once.
