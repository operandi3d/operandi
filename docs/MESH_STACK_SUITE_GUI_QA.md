# Mesh Stack suite: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi project-thread-fcjo2t <commit>.app`.
Open the Mesh Stack tab (right panel, last tab of the upper group).

## Batch 1: the stack still works (regression)

1. New scene, select the mesh. Add Operator ▾ › Cube, then Polygon Bevel.
   Expect a bevelled cube; dragging Inset in Properties updates it live and
   is one undo step.
2. Turn the Bevel's eye off and on; delete it; undo. Save, reopen: same
   result.

## Batch 2: Boolean

1. Make two meshes: A = Cube op (size 2), B = a plain 1-unit cube moved to
   (0.5, 0.5, 0.5).
2. Select A, then Shift-select B (A must stay the main selection). Add
   Operator ▾ › Edit › Boolean. Expect the Union of A and B; the
   Properties show Driver Surface = B.
3. Set Operation to Subtract: a corner bite. Move B with the Move tool:
   the bite follows. Hide B (eye in the Items list): the bite stays.
4. Untick Use World Transform and move A: the bite now stays at the same
   place on A.
5. The Driver Surface list doesn't offer A itself. Give B a Merge Meshes
   op and try to add A as its source: refused with "dependency loop".
   Delete B: A's Boolean row turns red ("Missing input"); undo brings the
   bite back.

## Batch 2: Merge Meshes

1. Three meshes: T (empty), S1 and S2 (cubes, moved apart). Select T, Add
   Operator ▾ › Edit › Merge Meshes. In Properties, Add Source › S1, then
   S2: both cubes appear in T where they are.
2. Index `1`: only S2. Index `0,1`: both. Index `x`: the row turns red.
3. Untick Source World Transform: the cubes jump to T's origin.
4. Parent a small cube under S1, tick Merge Hierarchy: it joins.
5. Delete S1: its copy disappears, the row stays normal; undo brings it
   back. Save, reopen: same result.

## Batch 3: selection ops

1. New mesh: Add Operator › Cube, then Polygon Bevel (Inset 0.1, Shift
   0.1, Group Polygons off). Every face bevels.
2. In the Bevel's Properties › Selection, Add Selection ▾ › Select by
   Previous Operation. Source Item = Cube, Name = Top: only the top
   bevels. Name = Caps: top and bottom. Name = Sides: the four sides.
3. Add Selection ▾ › Select by Pattern under it, Blend Mode Add, Pattern
   1,0: more faces join. Tick Invert, untick Enable: back to step 2.
4. Remove both; every face bevels again. Undo brings them back.
5. Save, reopen: same result.

## Batch 4: tools as ops (one pass over all seven groups)

Add Operator ▾ now has folders Polygon, Edge, Vertex, Duplicate, Edit,
Deform and UV. For each step: new scene, select the mesh, Add Operator ›
Cube first, then the op named; the cube changes live as you edit the op's
Properties, the eye turns it off, Undo removes it.

1. Polygon: Polygon Extrude (Extent Y 0.5), Spike (Strength 1),
   Detriangulate after Triangulate (back to 6 quads), Delete (cube gone).
2. Edge: Edge Bevel (Value 0.1, Round Level 2), Edge Chamfer (Offset 0.1).
3. Vertex: Vertex Bevel (Value 0.1): every corner cut.
4. Duplicate: move the cube's Position X to 3, then Radial Array: 8 cubes
   round Y. Clone (Count 3): a stack upward.
5. Edit: Axis Slice (Number X 2), Polygon Reduce on a Sphere op (Number
   100: exactly 100 polygons), Solid Drill with a second mesh selected
   (Shift-select it before adding): a bite out of the cube.
6. Deform: Jitter (Range X/Y 0.1, change Seed), Smooth (Lock Corner off).
7. UV: UV Unwrap then Pack UVs; open the UV view to see the islands.
8. Save and reopen one scene with several of these: same result.

## Add browser

1. Mesh Stack tab › Add Operator…: a 450×600 pop-over opens beside the
   button, Mesh Operations selected in Directories, the Name list grouped
   under folder headers (Mesh Operations/Create/Primitives, …/Polygon …).
2. Click Polygon in Directories: only Polygon ops. Back and Forward step
   between the two; Up goes to Mesh Operations.
3. Type "bevel": Edge, Polygon and Vertex Bevel (Search All). Switch to
   Search View with Polygon selected: only Polygon Bevel. Close and reopen:
   the search is cleared (its arrow lists it as recent).
4. Double-click Cube: it's added and the pop-over closes. Pin it, add two
   more: it stays. Click outside unpinned: it closes.
5. Drag the divider; drag it fully left: Directories hide (bottom-left
   button brings them back). Try the size slider and the four view modes.
6. An op's Properties › Selection › Add Selection…: the same widget as a
   plain list. Items list › Add Item (label): the Add Item browser; the ▾
   arrow still opens the menu.

## Tidy batch (2026-11-27)

1. Turn on a Linear falloff, add a Material Tag step, Add Selection…
   Select by Falloff: the row shows the falloff; drag Threshold and watch
   the tagged polygons change. Clear Falloff: nothing is tagged.
2. On a grid, an Edge step (e.g. Edge Bevel) with Select by Index (one
   edge) then Select Conditional Loop: the bevel runs border to border;
   turn Loop Both Directions off and flip Start Direction.
3. A UV-mapped plane as Target of a UV Transform step on a small mesh
   (Deformers folder), Target UV Map "Texture": the mesh lies on the
   plane; move the plane: it follows.
4. A curve with Curve Rebuild Spans: knots appear (Bezier); set Offset
   Mode Vector and Offset Amount: the new knots lift.
5. Lace Geometry (Duplicate folder) with a curve mesh as Guide Mesh,
   Radius 0.05: a tube along the curve; drag Start/End.
6. Wrap: run `deformer.rebind` from the command line with the step
   selected: same as pressing Rebind.
