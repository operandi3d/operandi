# MeshOps v1 GUI check

QA app identity only (`OPERANDI_DEV_DATA`). Claude can't click in
Operandi; this is for Jason or Codex.

1. New scene, select the empty Mesh. Upper side panel, **Mesh Ops** tab
   (last tab). Add Operator ▾ › Cube. A 1 m cube appears; the list shows
   Cube above Base Mesh.
2. Add Operator › Polygon Bevel (it lands on top and is selected). Its values show in the
   Properties panel; set Shift 0.1 there: every polygon bevels. Drag the Shift
   field: it updates live; one Cmd+Z undoes the whole drag.
3. Click Bevel's eye: the plain cube is back. Click it again.
4. Set Bevel's Shift to a huge value or Sphere Sides to 1: the row turns
   red, its tooltip and Properties give the reason, the ops above still
   run.
5. Drag Bevel below Cube in the list: the order changes (Bevel on an
   empty mesh does nothing, then the cube appears). Drag it back.
6. Try a direct tool (Polygon mode, Subdivide or Flip): refused with the
   Edit Base Mesh / Freeze message, and nothing changes.
7. Click **Base** in the tab's tool row (or right-click Base Mesh › Edit
   Base Mesh): the Base Mesh draws with the result as a light-blue ghost
   wire. Make a box there with the Box tool: the stack re-runs on it.
   Click Base again to leave.
8. Right-click Bevel › Freeze: Cube and Bevel bake into the Base Mesh, the
   list keeps the ops above. Cmd+Z brings the stack back exactly.
   Duplicate and Freeze makes a new item and leaves the original stack.
9. Save, close, reopen: same stack, same result.
10. Render (F9) and the Cycles Preview show the result, not the Base Mesh.
11. Add Mirror and Array on a Sphere and drag their values: the view stays
    responsive while results arrive (worker threads).
