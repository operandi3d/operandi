# Fast drags on big meshes: GUI check

Build: the QA app for branch `claude/project-thread-sbitxm`. Scene: any mesh
of about a million polygons (a Sphere subdivided until it has ~1M, or
`edit1m.opscene` from the perf snapshot).

1. Polygon mode, select one polygon in the middle, Move tool, drag the
   handle around for a few seconds. It should follow the pointer smoothly
   (around 40 fps on the Mac Studio), with no hitch when the drag starts.
2. Undo, then Redo. Each should show at once (the very first undo after
   opening may take under a second while picking is prepared).
3. Select about 10,000 polygons (a big lasso) and drag them: slower than
   one polygon, but steady.
4. Vertex mode: select a few points and drag; the selected points follow,
   no stray dots appear elsewhere.
5. Item mode: Move the whole item; then Polygon mode with nothing selected,
   Move (the whole mesh): both smooth.
6. Turn on Subdivision (Tab) and repeat step 1; the smooth surface follows
   the moved polygon and its neighbours.
7. Save, close and reopen the scene: the moved geometry is where you left it.

## Plain-mesh drawing and Item mode (2026-11-13)

8. Open a big plain (unsubdivided) mesh. Polygon mode: the wire looks as
   before (shared edges as dark as before, open borders lighter).
9. Press the Item-mode key, then back to Polygon mode, several times: each
   switch is immediate. Select and deselect the item in Item mode: also
   immediate; the selected item shows the selection fill and outline as
   before.
10. Lock a few polygons, then select the item: the locked polygons'
    wire still shows in the locked colour in Polygon mode.

## Many objects and the wire fade (2026-11-13)

11. Open a scene of thousands of objects (`objects10k.opscene` from the perf
    snapshot). Orbit: smooth. Click one object, then Select All and Move:
    all follow together.
12. The Items list scrolls smoothly and shows the right rows.
13. A very dense plain mesh, zoomed out: the wire fades instead of covering
    the mesh; zoom in and the wire comes back. Preferences > Display >
    Fade Wireframe on Small Polygons off brings the full wire back.
14. Type `pref.value display.fade_small_wire false` in the command line:
    the box unticks and the full wire is back.
15. Same dense mesh: its outline and any sharp creases keep their wire
    while the rest fades. Set the inactive group to Wireframe: nothing
    fades there.
16. Select a different item so the dense mesh is inactive: blue-grey
    brackets mark its box corners. Display > Colors > Geometry > Dense Mesh
    Box changes their colour.
