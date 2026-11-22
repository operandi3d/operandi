# Drag controls and Loop Slice drags: GUI checklist (2026-11-15)

For Jason or Codex, on the QA build, about five minutes. Spec
`specifications/loop-slice-2026-11-15`; the questions are QUESTIONS.md
sections B and C. Write down what you see; "felt wrong" is a fine answer.
The feel numbers (4 px axis lock, 12 px per whole step, 1 bar px per px)
are guesses waiting for this check.

Set up: a 3 × 3 grid (Plane, 3 segments each way), Edge mode, select one
edge, Edit tab ▸ Loop Slice (Alt+C). One slice appears at 50 %.

1. **Left-drag off the bar (LS-Q02).** Drag sideways anywhere in the view
   (on or off the mesh), slowly, about a hand's width. The slice follows,
   roughly 1:1 with the strip; right moves it toward the strip's right end.
   Too fast, too slow, wrong way?
2. **The bar still works.** Drag the triangle on the strip and the end dots
   as before.
3. **Click a different edge (LS-Q01).** A click without moving still picks
   that edge's ring (Shift adds a ring). Is that what you expect?
4. **Right-drag (LS-Q04, DL-Q02).** Drag sideways: one more slice every
   12 px to the right, fewer to the left. Too fast or slow?
5. **Middle-drag.** Sideways steps Current through the slices (watch the
   highlighted triangle). Up/down sets Inset: up digs the cut in, down
   raises it. Start a drag diagonally (DL-Q01): does it lock to the way you
   moved first?
6. **Ctrl and Shift (LS-Q05).** Left-drag with Ctrl, then Shift: today they
   change nothing. Did you want finer steps or snapping?
7. **Edit Add / Remove (LS-Q06), Mode Uniform / Symmetry (LS-Q09).** Try
   them on the strip and on the mesh; note what you expected.
8. **Another tool (DL-Q02, DL-Q03).** Polygon mode, select a polygon,
   Polygon Bevel: right-drag sideways changes Segments. Right = more?
9. **Trackpad (DL-Q04).** On the trackpad: Cmd+drag acts as a middle drag
   (Loop Slice Current), Cmd+Shift+drag as a right drag (Count). Comfortable,
   or would you pick other keys?
10. Type `tool.set poly.loopSlice on` once in the command line and check
    the tool starts (UIC-164).
