# Boolean GUI check (stage 1, branch booleans)

Use the QA build; your own scenes stay closed.

1. **BOOL-G01 Union.** Make two cubes that overlap at a corner. Select the
   first, Edit tab ▸ Booleans & Drill ▸ **Boolean...**, Operation Union, OK.
   The first cube becomes one solid (12 polygons, three of them L-shaped);
   the second cube is still there unchanged. Hide it to see the result.
   Cmd+Z brings the first cube back in one step.
2. **BOOL-G02 Subtract and materials.** Give the second cube another
   material, then Boolean ▸ Subtract. The corner is cut away and the cut
   walls show the second cube's material.
3. **BOOL-G03 Intersect with no overlap.** Move the second cube clear,
   Boolean ▸ Intersect. The first mesh is empty and the status line says
   the meshes don't overlap. Undo.
4. **BOOL-G04 Refusal.** Delete one polygon of the second cube, Boolean.
   A notice names it and says it has 4 open edges; **Select Problem Edges**
   selects those 4 edges on it. Nothing else changed.
5. **BOOL-G05 Drive Mesh.** In the dialog, Surface Item and Include
   Children grey out unless the Drive Mesh choice uses them. With Specific
   Surface Item, a hidden mesh can still drive.
6. **BOOL-G06 Add.** Boolean ▸ Add on the overlapping cubes: both shells
   stay, split where they cross (18 polygons).
