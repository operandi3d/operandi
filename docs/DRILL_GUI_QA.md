# Solid Drill and Axis Drill GUI check (branch drills)

Use the QA build; your own scenes stay closed.

1. **DRILL-G01 Solid Drill Core.** Make a Plane (4×4 segments) and a cube
   that pokes through its middle. Select the plane, Edit tab ▸ Booleans &
   Drill ▸ **Solid Drill...**, Operation Core, OK. Only the plane's middle
   is left; the cube is unchanged. Cmd+Z brings the plane back in one step
   (Edit menu says "Undo Solid Drill Core").
2. **DRILL-G02 Tunnel and Stencil.** Turn the cube 30°. Solid Drill ▸
   Tunnel: a slanted square hole, no walls. Undo, then Stencil with Material
   "Paint": the square stays, in a new material "Paint" in the Shader Tree.
   One Cmd+Z removes both the cut and the material.
3. **DRILL-G03 Dialog.** Material greys out unless Operation is Stencil;
   Surface Item greys out unless Drive Mesh is Specific Surface Item. The
   list reads Core, Stencil, Tunnel, Slice. Reopen: last values are kept.
4. **DRILL-G04 Axis Drill.** Make a cube and, above it, a small flat
   square (a 1-segment Plane). Select the cube, **Axis Drill...**, Axis Y,
   Tunnel: a square hole through the top and the bottom.
5. **DRILL-G05 Refusals.** Delete one polygon of the Solid Drill cube and
   run Solid Drill: a notice names it with 4 open edges; **Select Problem
   Edges** selects them. Axis Drill with the flat square turned on its edge
   (Axis Y) says the driver has no outline along Y. Nothing changed.
