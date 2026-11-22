# Reset Tool Attributes everywhere (MN-05): GUI checklist (2026-11-19)

For Jason or Codex, on the QA build.

1. With no tool active, Edit > Reset Tool Attributes is greyed; hovering
   says no tool is active. Cmd+D does nothing (no status change).
2. Select a polygon, Move (W), drag it. Cmd+D: the polygon goes back to
   where the drag started, Move stays active and its fields read 0.
   Edit > Undo reads "Undo Reset Move"; Undo puts the drag back.
3. Vertex Bevel, Inset, Edge Bevel, Twist Bevel: change a few values,
   Edit > Reset Tool Attributes. Values go back to the factory ones and
   the result updates at once.
4. Edge Bevel: set Segments 3, drop, start it again (it remembers 3).
   Reset: Segments goes to the factory 1, not 3.
5. Bevel, Extrude, Smooth Shift, primitives, Bridge, Spike: reset does
   what their own Reset Tool Attributes button did before.
