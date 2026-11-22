# Remesh GUI check (branch remesh)

Use the QA build; your own scenes stay closed.

1. **REM-G01 Voxel Remesh.** Make a Sphere and a Cube overlapping it in one
   mesh item. Edit tab ▸ **Remesh...**, Mode Voxel, Density 50 %, OK. A
   progress window shows briefly; the result is one even quad skin with no
   inner walls where they overlapped. Cmd+Z brings both shapes back in one
   step ("Undo Voxel Remesh").
2. **REM-G02 Density and Voxel Size.** Density 80 %: finer. Voxel Size 5 cm:
   quads about 5 cm whatever the Density. Reopen: last values are there.
3. **REM-G03 Sculpt.** Sculpt tab, paint a weight map, then **Remesh...**
   (opens on Voxel) with Keep Vertex Maps on: the painted area keeps its
   weights.
4. **REM-G04 Blocks / Smooth / Sharp.** On a Cube, Mode Sharp, Octree Depth
   5: corners stay sharp. Blocks: a stepped surface. Smooth: rounded.
   Octree rows grey out for Voxel; Sharpness only lights for Sharp;
   Threshold only with Remove Disconnected.
5. **REM-G05 Automatic Retopology.** On a dense Sphere, Edit tab ▸
   **Automatic Retopology...**, Polygon Target 2000: about 2000 quads that
   follow the shape. Relative 10 %: a tenth of the polygon count. New Mesh:
   a "… Retopo" item appears next to the original, which stays.
6. **REM-G06 Cancel.** Automatic Retopology with Polygon Target 200000 on a
   big mesh, then Cancel (or Esc): nothing changes; the status says
   canceled.
7. **REM-G07 Command line.** `sculpt.remesh` opens the dialog on Voxel.
   `mesh.retopo polyTargetNum:500` runs without it. `operandi.remesh
   depth:40` says Octree Depth must be from 2 to 12.
