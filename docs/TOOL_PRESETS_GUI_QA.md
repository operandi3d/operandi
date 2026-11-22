# Tool presets stage 1 GUI check

Spec: tool-presets v0.2 (2026-11-15). Use a 1 m cube made into a column
(Scale Y 3) with a few height segments, and a separate small cube.

1. **TP-G01 Deform presets unchanged.** Vertex mode, select the top half.
   Deform tab: Soft Move, Shear (Soft Drag flyout), Twist, Swirl, Taper,
   Bulge and Flex each start as before (status line names them, falloff
   handles appear), drag, drop. Each is one undo step named after it.
2. **TP-G02 Your falloff comes back.** Pick Falloff ▸ Radial from the
   context bar and move it aside. Start Soft Move, then Shear straight
   from it, drop: your Radial is back where you left it. Do it again with
   Falloff ▸ None first: no falloff is left afterwards.
3. **TP-G03 Change while open.** Start Taper, pick a different falloff
   from the pie, drag, drop: your original falloff comes back (Jason
   11-15: restore on drop).
4. **TP-G04 Flex gives back the pivot.** Set Action Center ▸ Origin. Polygon
   mode, select a ring of polygons, Flex (Shift+Y), bend, drop: Action
   Center is Origin again.
5. **TP-G05 Flare.** Bulge flyout ▸ Flare on the column with the top half
   selected: drag right; the top swells out, the middle half as much, the
   bottom of the selection not at all. Drop: falloff and action centre are
   yours again. `tool.set Flare on` in the command line does the same.
6. **TP-G06 Thicken Reset.** Polygon mode, select the top, Thicken, drag,
   then Reset in Tool Properties: Thicken stays on with Max Smoothing
   Angle 90.1° (it used to switch to Smooth Shift's values).
7. **TP-G07 Flip It.** Polygon mode, select the small cube's polygons
   (double-click one). Polygon tab ▸ Commands ▸ Flip It...: a small dialog
   with Axis X / Y / Z, on X. Cancel: nothing changes. Flip It... again,
   Y, OK: the cube flips upside down in place and still shades as solid
   (facing out). Cmd+Z undoes it in one step.
8. **TP-G08 Flip It Origin.** Move the small cube to X = 2, Flip It
   Origin... ▸ X ▸ OK: it lands at X = −2, facing out. With Symmetry on it
   does the same (symmetry is ignored).
