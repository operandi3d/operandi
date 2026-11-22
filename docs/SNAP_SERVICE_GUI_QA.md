# Shared snapping service — GUI check

Branch claude/project-thread-ofjqsa (architecture review pick B). Nothing
should look different except item 3. Use the QA build with its own data
folder.

1. Move a point with snapping on (X): the handle still jumps onto a point
   within the Inner Range and pre-highlights within the Outer Range.
2. Pen and Curve: placed and dragged points still snap to mesh points and
   to the draft's own points.
3. Behind the surface: with a cube in front of a plane, drag a handle, a
   Curve point and a Ruler end over a plane point that the cube hides. It
   must not snap (it used to for Curve-with-snapping-off and the Ruler).
4. Big mesh: on a mesh over 200,000 points (Sphere, high segments), a
   Curve point with snapping off and a Ruler end still join a point.
5. Weld: drag a point onto a neighbour within about 15 px; it highlights
   and welds on release.
6. Symmetry on, Pen: a point placed within about 8 px of the plane lands on
   it.

## Stage 2 (snapping spec v2.2)

7. F11 over a view: the Snapping popover opens at the pointer inside the
   window, with no title bar; moving the pointer off it, Esc, or F11 again
   closes it. The circle at its top right pins it; once pinned, drag the
   strip along its top (the pointer turns into a hand) to move it anywhere
   in the window. It can't be dragged out of the window.
   It comes back there after a relaunch.
8. Rows top to bottom: Snapping button, Snap Types (Mode, Preset, twelve
   ticks), the option groups for what is ticked, Inner/Outer Range
   (pixels), Coordinate Rounding, Fixed Increment (greyed unless Fixed or
   Force Fixed), Snap Angle. No round markers beside the rows.
9. Tick Edge and drag a Move handle near an edge: it lands on the edge.
   Try Edge Center, Polygon Center, Box (corners only, and the corners of
   one box around every mesh) and Pivot the same way. A small reticle in
   the Unsnapped colour shows where the point would be without snapping.
10. Tick Polygon with Vertex: over a face the handle lands on the surface,
    near a corner on the corner.
11. Preset: (New Preset), type a name, Add; change ticks, close, reopen: the
    preset kept them. Pick Default: its ticks come back.
12. Pen with Straight Line ticked: a new segment close to the line of the
    one before locks onto it.
13. Move an axis arrow with Vertex snapping and move the pointer onto a
    point off the axis: the reticle follows the pointer, and the handle
    lands at that point's position along the axis.
14. The toolbar Snapping button always reads "Snapping" and lights up while
    snapping is on. Hovering it lists the ticked snap targets, one per
    line.
15. Service holdouts (2026-11-20), Vertex snapping on, a grid mesh:
    Pen Slice clicks and drags land on points; with World Axis ticked a
    press-drag within Inner Range of a world axis line locks onto it (a
    plain click never locks) (Pen Slice's own World Axis /
    Straight Line / Right Angle ticks are gone). Draw a Cube from near one
    point to near another: both corners land on them; haul a side near a
    point: it stops level with it. Create (Vertex mode) clicks land on
    points. Solid Sketch: a new node and a dragged node land on another
    mesh's points, never on the sketch's own skin; Drop, open Solid Sketch
    again (Yes) and click near a point: no crash, it snaps.
16. Radial Array / Radial Sweep with snapping on: the arc's end cubes step
    by Snap Angle (Rotation Snap Angle) without Ctrl. Mirror's hinge with
    Ctrl steps by Snap Angle too (no longer a fixed 15°).
17. Symmetry X, Edge mode: select an edge on the − side, then its mirror
    on +, open Edge Bevel; repeat picking them the other way round: the
    handle sits on the same (+) edge both times.
