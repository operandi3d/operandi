# Curve Boolean — GUI check

Build: the QA app in ~/Developer/Operandi-Builds named in the thread. Fresh
scene. "Left-click" = left mouse button; "Shift+left-click" adds to the
selection. Keys: 3 Polygon mode, Q drops a tool.

1. Make an empty mesh (Items list: New Mesh). Curve tab > Shapes:
   left-click **Rectangle**. In its properties set Type **Polyline**,
   Size X and Y 2, Position X and Y 1. Left-click **Rectangle** again and
   set Polyline, Size 2 × 2, Position 2, 2. Then freeze the two operations
   (the Mesh Operations list's Freeze) so both squares are plain closed
   polylines in the mesh: Curve Boolean, like every direct tool, won't
   edit a mesh that still has operations.
2. Press 3 (Polygon mode). Left-click square A (the one at 0..2) to select
   only it.
3. Edit tab > Booleans & Drill: left-click **Curve Boolean...**. The dialog
   shows Operation (Slice first), Axis Auto, Clip off, Merge Curves on.
   Pick **Union**, left-click OK: A becomes the L-shaped outline round both
   squares; B is unchanged; nothing is selected. Cmd+Z: back in one step.
4. Repeat with **Intersect** (A becomes the overlap square), **Subtract**
   (A becomes the notched square) and **Slice** (A looks the same but has
   two new points where B crosses it: press 1 to see them).
5. Union with **Clip** on: A's outer part stays as an open line (no B
   segments).
6. Subtract with **Merge Curves** off: two open pieces instead of one
   outline.
7. Select both squares (left-click A, Shift+left-click B), Union: one
   outline round both, and both originals are gone.
