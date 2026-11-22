# Key toggles (MN-02): GUI checklist (2026-11-19)

For Jason or Codex, on the QA build. Polygon mode, a polygon selected.

1. Hold W, drag in the 3D view, let go of the mouse, let go of W: the
   polygon moved, Move is off, you're back in plain selecting. One Undo
   takes the move back ("Undo Move").
2. Tap W (no drag): Move turns on and stays on, as before.
3. Hold W, start a drag, let go of W before the mouse: Move stays until the
   mouse comes up, then drops.
4. With Move already on, hold W, drag, release: Move stays on.
5. Hold W for a few seconds without moving (key repeat): Move doesn't
   flicker on and off.
6. Rotate (E) on, hold W, drag, release: Move did the drag, Rotate is back.
7. Same with a tablet pen stroke instead of the mouse.
8. Preferences > Input > Keys > Key Toggles off: hold W and drag, release:
   Move stays on (old behaviour). Survives relaunch.
