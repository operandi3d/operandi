# Edit tab wave 1: GUI check (Subdivide, Center, Slide, Freeze...)

Spec: `docs/specs/edit-wave1/SUBDIVIDE.md`. QA app:
`~/Developer/Operandi-Builds/Operandi subdivide-tools <commit>.app` (its own
data folder; don't use your installed app or scenes).

Start each block from a new scene with a Cube (Create tab), then open the
**Edit** tab.

## Layout and icons

1. Top of the Edit tab: a large **Slide** button (greyed in Polygon and Item
   mode), then Slice, then **Center** (X, Y, Z, All in one row), then
   **Subdivide** (SDS Subdivide, Faceted, Smooth, Options..., UnSubdivide...,
   each with an icon), a divider, **Freeze...**.
2. Hover each button: name, shortcut and the modifier alternates show
   (SDS Subdivide: "Option: SDS Subdivide 2X"; Options...: Shift+D;
   X: "Option: XY").
3. Hold Option: SDS Subdivide reads "SDS Subdivide 2X", X/Y/Z read XY/YZ/ZX.
   Let go: back to normal.
4. Do the six new icons read well at toolbar size, light and dark theme?
   (They need your OK.)

## Subdivide

5. Polygon mode, nothing selected, click **SDS Subdivide**: the cube becomes
   24 quads and shrinks into a rounded box. Cmd+Z: back to the cube in one
   step.
6. Option+click **SDS Subdivide**: 96 quads, rounder; one Cmd+Z undoes both.
7. Select one polygon, press **D**: only it is split into four, the four new
   polygons are selected, its neighbours get an extra point on the shared
   edge (they become 5-sided).
8. **Faceted** on the cube: 24 quads, shape unchanged.
9. **Smooth** on the cube: looks like Faceted (all cube edges are 90°, above
   the 89.5° default). Then Shift+D, Max Smoothing Angle 91, Method Smooth,
   OK: the faces bulge out.
10. **Shift+D** in Polygon mode (or the **Options...** button): the
    Subdivide dialog: Subdivision Method, Fractal, Max
    Smoothing Angle, Linear UVs, Boundary Rules and Ignore Subdivision Edge
    Weights (both greyed unless Catmull-Clark), Background Constraint greyed.
    Set Fractal 20 %, OK: the new points jitter. Run again: a different
    jitter. Escape/Cancel changes nothing.
11. After step 10, click **Faceted**: it keeps Fractal 20 % (buttons only
    change the Method). Quit and relaunch the QA app: Options... still shows
    the last values.
12. Vertex mode with a point selected, press D or click SDS Subdivide:
    nothing changes, the status bar says "No polygons selected".
13. Edge mode with an edge selected, press **D**: Edge Subdivide as before
    (a point in the middle of the edge). Shift+D in Edge mode: the dialog.
13a. Make a second cube item. Item mode, select both cubes, press **D**:
    both become 24 quads; one Cmd+Z undoes both.
13b. Item mode, **Shift+D**: nothing changes; the status bar says
    "Instance isn't available yet".

## UnSubdivide

14. Cube, SDS Subdivide, then click **UnSubdivide...**: a dialog opens first
    (Iteration 1, Flat off, UV and Material Corner Hint on, the other hints
    off, Selection Set Name greyed until its hint is on, with a ▾ list of the
    mesh's polygon sets). Esc/Cancel: nothing changes. OK: the exact cube
    comes back (6 polygons). Reopen: your last values are still there.
15. Cube, SDS Subdivide twice, UnSubdivide... with Iteration 2: the cube again.
16. Make a 4x4 grid (plane with 4 segments), select the middle 2x2, SDS
    Subdivide (those 16 quads stay selected), UnSubdivide... OK: four
    polygons come back and stay selected. Each keeps the extra points its
    neighbours share (six corners, edges straight), the neighbours are
    unchanged (5-sided), and there are no cracks or folds.
16a. A sphere (or cube) SDS Subdivided whole, then select a patch of quads
    and UnSubdivide...: the patch merges back, its outline polygons become
    n-gons where they meet unselected polygons, and nothing folds over or
    cracks.
17. Cube, Faceted, UnSubdivide...: 6 polygons and the cube grows (expected).
18. A cube with one polygon Tripled (a triangle), UnSubdivide...: nothing
    changes; "UnSubdivide only rebuilds quads". Vertex mode with a point
    selected: "No polygons selected".

## Center

18. Move the cube's polygons away from the origin (Polygon mode, select
    all, Move). Click **X**: the selection's centre sits at X = 0; Y and Z
    unchanged. Option+click **Y** (YZ): centred on Y and Z. **All**: on all
    three. Each is one undo step.
19. Item mode, click **X**: nothing moves; the status bar says to switch to
    a component mode.

## Slide and Freeze...

20. Edge mode, select an edge, click **Slide**: Edge Slide starts. Vertex
    mode with a point: Vertex Slide starts.
21. Cube, Shift+Tab (Catmull-Clark surface), **Freeze...**: the dialog
    lists Freeze Curves as (Faces), Make Holes, Drill Axis, Freeze
    Displacement, Freeze Deformation, Tessellation, Subdivide Curves with
    Fixed Spans, Curve Refinement Angle (5 deg), Save Deformation to Morph,
    Morph Map Name. Only Tessellation can be changed; the rest are greyed,
    readable, and say "Not available yet" on hover.
22. Tessellation on, OK: the smooth surface becomes plain polygons in one
    undo step. Undo, Freeze... again with Tessellation off, OK: nothing
    changes (the status bar says the cage stays). Reopen: still off.
    Cancel changes nothing.
23. Dialog look (all three dialogs): labels right-aligned in a left column,
    fields beside them, OK (Return) and Cancel (Esc) in a column on the
    right. This look is interim (SUBT-Q14).

## Big results (2026-11-12)

24. New Sphere, Subdivide five times: every pass draws; no crash (the
    5th pass used to abort the app).
25. Keep subdividing: no fixed 8 million stop. When a pass would need more
    than half of the free memory, a **Subdivide Polygons** question gives
    the polygon count, the memory it needs and what is free. Return
    subdivides (one undo step); Escape changes nothing. A pass needing
    more than all of the free memory is refused with those numbers in the
    status bar.
