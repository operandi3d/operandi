# Curve tools (path tools, Curve tab, 2D shapes) — GUI check

Build: the QA app in ~/Developer/Operandi-Builds named in the thread
(`OPERANDI_CYCLES=1`). Use a fresh scene. Every step that changes the mesh
should come back with one Cmd+Z after the tool drops.

## Path tools (Duplicate tab)

1. Add a Cube, Polygon mode, select its top face. Duplicate tab >
   **Curve Extrude**. In the Front view click three points going up and
   to the right. The top face sweeps along a smooth curve as you click;
   the start is open and the end is capped. Drag a point: the sweep
   follows. Space drops it; the end cap is selected.
2. Same with **Bezier Extrude**: each click adds an anchor with handles
   (round dots); drag a handle to bend the path. **B-Spline Extrude**:
   the path stays inside its points.
3. Tool Properties: Steps 4 makes 4 rings; Automatic adds rings where the
   path bends; Align to Path off keeps every ring flat; Cap Start on keeps
   the bottom; Size 50 % tapers the end; Twist 90° twists it.
4. Mode **Sketch**: drag a freehand stroke. It becomes a clean path of a
   few points and the sweep follows it.
5. Mode **Delete**: click a point to remove it. **Clear Path** starts
   again. Esc cancels the whole tool; the cube is as it was.
6. Symmetry X on, select two mirrored faces, Curve Extrude: both sides
   sweep, mirrored.
7. Select a small cube's polygons, **Pen Clone**: click three points.
   Copies sit with their bottoms on the second and third points and turn
   to follow the line (halfway at the bend). Align off: upright copies.
8. **Curve Clone** with Steps 7 along an arc: seven copies (the original
   counts), turned along the arc. By Length spaces them evenly.
9. Click on another mesh's (Sub-D) surface: the path point lands on it.
   Snapping on (X): points snap to vertices.

## Curve tab

10. Vertex mode, click four points in the order you want, Curve tab >
    **Make Closed Curve**: one closed curve through them in that order;
    the mode switches to Polygon with the curve selected. With one point
    selected the status says "Select two or more vertices".
11. **Make Curve Fill** on that curve: a flat polygon fills it; the curve
    stays. On an open curve the status line refuses.
12. Draw two closed curves going opposite ways, select both, **Align
    Winding**: both now run the same way (check with Make Curve Fill: both
    fills face the same way).
13. **Set Curves Color…**: Override on, pick red, OK. Curves draw red.
    Save the scene: the file doesn't change; Undo has no step. Override
    off: back to the usual colour.
14. The Offset and Convert rows and Bezier Tangent say "not built" (they
    wait for the hands-on batch).

## 2D shapes (Mesh Stack)

15. Curve tab > Shapes > **Circle**: a Circle op appears in the Mesh
    Operations list with Radius 0.5 on the XZ plane. Change Radius, Type
    (Spline), Axis: the curve updates live.
16. Same for Ellipse (1 × 0.5), Rectangle (1 × 1), Diamond, N-Sided
    (8 sides) and Star (5 points, Scale 50 %).

## Batch 5

17. Select a small cube, Duplicate tab > **Scatter Clone**, click in the
    view: 8 cubes (the original counts) scattered within ±1 m. Change
    Seed: a different set; Range Y 0: all on the ground. Space drops it.
18. Mesh Stack: add **Scatter Clone** to a small cube's stack, Count 9: nine
    cubes spread evenly over a 1 × 1 square from the origin.
19. Make a floor plane and a wall grid above it. Add **Surface Constraint**
    to the wall, pick the floor as Constraint Surface, turn on Invert:
    the wall flattens onto the floor. Strength 50 %: halfway.
20. UV tab > **UV Rectangle** on a quad grid with messy UVs: every cell
    becomes a clean rectangle; triangles nearby stay as they were.
21. Two copies of a mesh, the target in front: type
    `operandi.vertMap.transfer type:position` with the other shown as a
    background mesh; the target's points jump onto it.
22. Add a Cube, Edit tab > **Julienne…**: Number 3, Offset X 0.25, OK.
    The cube is four closed slices with small gaps; Cmd+Z puts it back.
23. **Dicer…** on a cube, Seed 40, OK: closed chunks. Run again with the
    same seed after Undo: the same chunks.
