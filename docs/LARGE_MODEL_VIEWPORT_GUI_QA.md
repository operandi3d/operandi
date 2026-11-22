# Large-model viewport GUI checks (2026-11-20)

Branch `large-model-perspective`. One pass with the QA app, any heavy scene
(not a model shared privately with the project).

1. **Maximise keeps the lens.** Quad layout, Perspective view: note how a
   box looks. Maximise (0), Restore, Maximise again: the box keeps its size
   and the perspective never gets stronger; the grid doesn't fan out.
2. **Old layouts heal.** Launch with a layout saved by an older build whose
   Perspective view had been maximised: it opens with a normal-looking lens.
3. **Field of view.** Over the Perspective view, hold Alt+Shift and roll the
   wheel: the perspective gets flatter one way and stronger the other, the
   orbit centre keeps its size, and "Field of View 58° · 22 mm" (changing)
   shows at the top of the view, then fades after about 1.5 s. The plain
   wheel still zooms. In Top, Front or Camera view it changes nothing.
   Quit and relaunch: the view keeps its field of view.
   Alt+Shift+middle-click puts it back to 58° · 22 mm with the same
   readout, the model keeping its size (not in the Maya preset, where
   Alt+middle pans).
4. **No polygon question.** Open a scene of 8M+ triangles and edit it: no
   Max GL Polygon Count dialog, and no Max GL row in Preferences > Display.
5. **Graphics memory note.** Only a scene near the card's memory budget
   shows "Graphics memory full: n items drawn as boxes" at the lower right
   and boxes its heaviest items; the mesh being edited stays solid.
