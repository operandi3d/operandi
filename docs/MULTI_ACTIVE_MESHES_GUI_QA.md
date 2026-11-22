# Multiple active meshes, stage 1: GUI check

Every selected mesh is now "active": in Vertex, Edge and Polygon modes they
all show their points, edges and polygons, and you can select components on
any of them. The first mesh you selected stays the **primary**; tools still
only edit the primary for now.

Setup: start the QA build and make two cube meshes side by side (a Cube,
then New Mesh and another Cube, moved aside in Item mode).

1. Items list: click Cube 1, then Cmd-click Cube 2 (Ctrl-click works too).
   Both are highlighted. Cmd-click Cube 2 again: it deselects; once more
   brings it back. Opt-click is a plain click.
2. Press 3 (Polygon mode). Both cubes draw in the active style (not the
   dimmer inactive one).
3. Click a polygon on Cube 2. It highlights. In the Items list Cube 1 is
   still the first selected item (the primary).
4. Shift-click a polygon on Cube 1. Both polygons stay highlighted. The
   status line says "2 selected Polygon".
5. Ctrl-click the Cube 2 polygon. Only that one goes away.
6. Plain-click another polygon on Cube 2. The Cube 1 polygon clears; only
   the new one is selected.
7. Click empty space. Nothing is selected, and both cubes are still
   selected in the Items list.
8. Press 1 (Vertex mode): both cubes show their points. Click a point on
   each (Shift for the second). Press 2 (Edge mode): click an edge on each.
   Press 1 and 3 again: each mode remembers what you picked on each cube.
9. Lasso around both cubes in Polygon mode: polygons on both are selected.
10. Select > Select All (or the shortcut): every polygon on both cubes.
    Then Invert, Grow/Shrink, Loop/Ring, Convert to Vertices: each works on
    both cubes.
11. Edge mode: double-click an edge on Cube 2. Its loop goes to Cube 2's
    selection.
12. Undo (Cmd+Z) a few times, then Redo: each cube's selection comes back
    the way it was.
13. Save the scene, close it, open it again: both cubes are still selected
    with the same components.
14. Cmd-click Cube 2 in the Items list to deselect it: its components
    clear. Cmd-click it back: it starts with nothing selected.
    In the viewport in Item mode, Shift-click adds a mesh and physical
    Ctrl-click toggles it (Cmd-click there still opens the modeling menu).
15. Tools edit only the primary: select polygons on Cube 2 only (nothing on
    Cube 1), pick the Move tool, then drag. Nothing moves, and the status
    line says tools edit only the primary mesh. Select a polygon on Cube 1
    too and drag again: only Cube 1's polygon moves.
16. Preferences > Selection, "Pick Other Meshes in Component Modes" off
    (the default): clicking a third, unselected mesh in Polygon mode does
    nothing to the selection. On: it makes that mesh the only active one.

Report anything that looks wrong with the step number.
