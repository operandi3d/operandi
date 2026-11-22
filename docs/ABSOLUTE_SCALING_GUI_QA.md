# Absolute Scaling — GUI check

Build: the QA app named in the thread. Use a fresh scene.

1. Create tab > Measure > Snaps and Precision, then Absolute Scaling in
   the Snaps panel: a window titled "Absolute" opens and the button lights.
   Click again: it closes. (It is no longer on the Measure shelf.)
2. Add a Cube. In the window press **Grab Size**: X, Y, Z and Uniform show
   the cube's size. Nothing moves; Undo has no new step.
3. Type Uniform 3, press **Uniform Scale**: the cube is 3 on every side,
   about its middle. Cmd+Z puts it back in one step.
4. Type X 4, Y 0, Z 1, X Center "Low X", press **Explicit Scale**: the cube
   is 4 wide, growing from its left side; Y unchanged (0 leaves it alone);
   Z is 1. The fields show the new size.
5. Polygon mode, select only the top face. Explicit 2, 5, 2: the face
   scales in X and Z; it stays flat, and the status line says Y was left
   unchanged.
6. With the top face still selected, press **Reference Scale** with X 4:
   the whole cube scales on X by the face's factor.
7. Rotate a box 45° about Y with Rotate in Polygon mode (components, not
   the item), turn on **Orient To Selection**, press Grab Size: the sizes
   are the box's own sides, not the larger axis-aligned ones.
8. Type `scale.absolute mode:explicit absolute_size_x:2` on the command
   line: the selection becomes 2 wide and the window's X shows 2.
9. Make a box whose origin is at its base (move the geometry up in Polygon
   mode). Set X/Y/Z Center to "Origin X/Y/Z" and Explicit-scale it: it
   grows from the origin; the origin stays at its base.
