# Sculpt brushes respect the component selection (SCS-01..09)

GitHub issue #4.

- SCS-01 In Vertex, Edge or Polygon mode, when the mesh has components of that mode selected, sculpt brushes move only the points of the selection: selected vertices; both ends of each selected edge; every corner of each selected polygon.
- SCS-02 Nothing selected in the current mode, or Item mode: the brush acts on the whole mesh (today's behaviour).
- SCS-03 Several active meshes: if any has a selection in the current mode, a mesh with none is left alone. If none has one, all are sculpted.
- SCS-04 Hard edge. Selected points take the brush's full weight (tip, falloff, pressure as usual); unselected points never move. No extra fade at the selection border.
- SCS-05 Applies to every brush: Shift-smooth, Ctrl-invert, Move/grab, symmetry mirrors, and the Vertex Map (weight) brushes.
- SCS-06 Unselected points still count as neighbours (Smooth averages against them).
- SCS-07 Symmetry: a mirrored dab moves only selected points; an unselected mirror partner stays put.
- SCS-08 Locks, hidden and pinned points stay frozen as before; the selection only adds to what is frozen.
- SCS-09 No new UI. One stroke = one undo step, as before.
