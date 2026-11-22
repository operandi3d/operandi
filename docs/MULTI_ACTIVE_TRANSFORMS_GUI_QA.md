# Multiple active meshes, stage 2 (transforms): GUI check

Move, Rotate, Scale and Transform now work on every selected mesh at once.

Setup: start the QA build and make two cube meshes side by side (a Cube,
then New Mesh and another Cube, moved aside in Item mode). Select both in
the Items list (click one, Cmd-click the other).

Component modes
1. Polygon mode: click the top polygon of each cube (Shift-click the second).
2. Pick Move. One handle appears, halfway between the two tops.
3. Drag the handle up. Both tops rise together.
4. Undo (Cmd+Z) once. Both tops go back in one step. Redo brings both back.
5. Ctrl-click the first cube's top to deselect it. Drag Move again: only the second cube's top moves.
6. Select nothing (click empty space) and drag Move: both whole cubes move.
7. Rotate and Scale with a polygon selected on each: they turn and grow
   about the one handle between them, not each about its own centre.
8. Vertex and Edge modes: pick a point or edge on each cube and Move; both
   move.
9. With a falloff on (Linear from one cube to the other), Move: the cube
   at the full end moves more than the one at the other end.

Item mode
10. Item mode with both cubes selected: pick Move. The handle sits halfway
    between the two items' positions. Drag: both items move.
11. Rotate: the two cubes swing round the shared handle. Scale: they grow
    and spread out from it. Undo puts both back in one step.
12. Lock one cube (Properties > Assembly > Lock), Move again: the locked
    one stays, the other moves.
13. Parent one cube under the other (drag it onto it in the Items list),
    select both, Move: the child moves once, with its parent, not twice.
14. With one item selected, Rotate and Scale work about that item's own
    position, as before.

Report anything that looks wrong with the step number.
