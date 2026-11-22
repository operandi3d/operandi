# Multiple active meshes, stage 3 (tools): GUI check

Mesh-editing tools and commands now work on every selected mesh at once,
each using its own selection, as one undo step.

Setup: start the QA build and make two cube meshes side by side (a Cube,
then New Mesh and another Cube, moved aside in Item mode). Select both in
the Items list (click one, Cmd-click the other). Polygon mode.

Polygon tools
1. Click the top polygon of each cube (Shift-click the second).
2. Pick Bevel. One handle appears, halfway between the two tops.
3. Drag the Shift handle up. Both tops bevel up together, by the same
   amount. Drop (Space). Each cube's new top stays selected.
4. Undo (Cmd+Z) once: both bevels go in one step. Redo brings both back.
5. With both tops selected, try Extrude, Inset, Shift and Thicken: both
   cubes change each time. Bevel: Shift-click a cap to continue; both
   continue.
6. Start Bevel, drag, then Esc (Cancel): both cubes go back as they were.
7. Ctrl-click the first cube's top to deselect it. Bevel again: only the
   second cube changes, the first stays as it is (no refusal message).
8. Scale the second cube up 2x in Item mode, back to Polygon mode, both
   tops selected, Shift 0.5 m: both tops rise 0.5 m in the viewport.

One-shot commands
9. With a polygon selected on each cube: Flip, Triple, Subdivide (Edit
   tab) and Remove each change both cubes; one Undo undoes both.
10. Select nothing (click empty space) and run Flip: every polygon of both
    cubes flips.
11. Select two touching polygons on the first cube and one on the second,
    run Merge: it refuses with a message naming the second cube, and
    nothing changes on either.
12. Set Material with a polygon on each: both get the material.

Edge and vertex tools
13. Edge mode: pick one edge on each cube. Edge Bevel and Edge Extrude
    change both.
14. Vertex mode: pick a corner on each cube. Vertex Bevel changes both.

The rest
15. Axis Slice with 1 cut in Y: both cubes are cut through the middle.
16. Tweak: press on a point of the second cube (not the primary) and drag:
    that cube's point moves. The hover outline shows on whichever cube is
    under the pointer.
17. Polygon on each cube, Lock Selected (J): both polygons lock. Hide
    Selected (H) hides both.
18. Polygon on each, assign a Selection Set, clear the selection, then use
    the set: both cubes get their polygon back.

Still on the first-selected mesh only (expected): Loop Slice, Edge Slice,
Poly Haul, Add Edge Point/Loop, Weld, Cut/Copy/Paste, Bridge, Spike,
Reduction, Pen, shapes. With nothing selected on the first mesh they say
so instead of editing it whole.

Report anything that looks wrong with the step number.
