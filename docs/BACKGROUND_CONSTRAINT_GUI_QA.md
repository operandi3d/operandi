# Constrain to Background: GUI check

For Jason or Codex (Claude can only watch). Specs: SPEC-BACKGROUND-CONSTRAINT
v1 (tutorial gaps 2026-11-13) and BEHAVIOR-MESH-CONSTRAINT v1
(docs/specs); questions BG-Q1..Q12, MC-Q1..Q4.

1. Make a sphere in one mesh item ("Base"). New Mesh: the new item is
   active, Base stays visible and inactive (drawn as background).
2. Click the Mesh Constraint button in the toolbar (beside Snapping): it
   turns orange and reads "Background". Tool Properties opens with a
   "Constrain to Background" section (Geometry Constraint Screen Axis,
   Offset 0 m, Negative Offset ticked, Handle Constraint ticked and
   greyed, Double Sided, Target Item). Snapping didn't change.
3. Front view, Polygon mode, Pen: click a few points over the sphere. In
   Perspective every point sits on the sphere's front, not on the work
   plane.
4. Drop, Vertex mode, Element Move: drag a point in Perspective. It slides
   over the sphere and never leaves it. Points not dragged stay put.
5. Offset 0.05 m in Tool Properties: the dragged point floats just off the
   surface. Geometry Constraint Off greys the other rows.
6. Drop the tool, Esc over the view: the constraint is gone (button reads
   "Mesh Constraint", not orange; section gone). Element Move now moves
   the point freely. Click the button again: Offset is still 0.05 m.
7. Undo after a constrained drag: one step, point back where it was.
8. SubD background (BG-Q5): make a cube as Base, Tab so it shows smooth.
   New Mesh, Front view, Pen: click over a front corner of the cube. The
   point lands on the rounded surface, visibly inside the cage corner, not
   on the cage. Tab Base back to Face: a new point over the same spot lands
   on the flat cage face. Change Base's subdivision level: points land on
   the new surface.
9. Popover: hold Option over the button (it reads "Options") and click:
   the Mesh Constraint popover opens at the pointer. Rows: the pin circle;
   Constraint Mode strip (none) · Guide · Background · Primitive (Guide
   and Primitive greyed, "Not built yet"); while Background is lit, the
   Constrain to Background rows, without Target Item. Click (none): the
   constraint turns off and the rows go. Click Background: on again.
   Right-click on the button and Edit ▸ Snapping ▸ Mesh Constraints… open
   it too. The Snapping popover has no constraint rows, and Edit ▸
   Snapping no constraint toggle.
10. Pin the popover, drag it by its top strip, quit and relaunch: it comes
    back pinned in the same place, and the options are the ones you left
    (the constraint itself starts off). Opening the Snapping popover
    closes an unpinned Mesh Constraint popover, and the other way round.
11. Snapping and constraint together: snapping on (Vertex target), the
    constraint on, Front view, Move a point of the active mesh near a
    point of another active polygon: it snaps to that point's spot on
    screen and lands on the background surface behind it.
