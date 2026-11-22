# Snaps and Precision — GUI check

Use a fresh scene with a Cube. Snapping starts off.

1. Create > Measure: "Snaps and Precision" opens the "Snaps" palette; the
   button lights. Edit ▸ Snapping lists Snaps and Precision first and also
   toggles it.
2. Palette order: Presets (Drag Snap, Drag Snap Rigid, Planar Rotate Snap,
   Planar Rotate Snap Axis, Scale Absolute, Size Absolute, Advanced
   Handles), then Commands (Absolute Scaling).
3. **Drag Snap**: snapping turns on and Element Move starts. Drag one cube
   corner near another: it lands exactly on it; nothing else moves. Undo
   is one step. Press Drag Snap again: the tool drops, snapping stays on.
4. **Drag Snap Rigid**: add a second cube in the same mesh (or Duplicate).
   Drag a corner of one onto a corner of the other: the whole cube moves
   with it, the other stays.
5. **Scale Absolute**: click a cube corner (the centre goes there). Drag
   the red circle onto the next corner along X, then the red square onto a
   point twice as far: X scales ×2 exactly. Tool Properties shows Scale
   Factor and Absolute Size.
6. **Size Absolute**: same as 5; Tool Properties shows Absolute and
   Relative. Type Relative X: the cube's X scales to Relative ÷ Absolute.
7. **Planar Rotate Snap**: click a face. Drag the outer circle onto one
   corner of that face, then the inner dot onto another: the selection
   turns about the face's normal so the first direction meets the second.
8. **Planar Rotate Snap Axis**: click a face, drag the two white dots onto
   two corners of an edge: the rotation now turns about that edge.
9. **Advanced Handles**: the button sets Tool Handles to Advanced and
   greys out. Plain Rotate now also shows the outer/inner handles.
   `tool.handleStyle default` puts it back.
10. Absolute Scaling in the palette opens and closes the Absolute window.

## Floating palettes (Jason, 2026-11-10)

11. Snaps, Absolute, Snapping Options (F11, opens at the pointer), Work
    Plane Settings, Diagnostics and Preview Settings (inside the Preview)
    each open as their own small window. Drag each outside the main window;
    it stays on top and works there. Close it with its red button; the
    menu or button opens it again.
