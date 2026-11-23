# Toolsets rework GUI checks (2026-11-23)

1. [ ] The toolset bar shows Model, UV, Brush, Setup, Animation, Workbench,
       Game as text buttons. Clicking each changes only the toolbox: no
       view opens, closes or resizes (UV no longer opens a UV view).
2. [ ] Model: Create, Select, Deform, Duplicate, Edit, Vertex, Edge,
       Polygon, Topology, Curve (no Sculpt, UV or Workbench tab).
3. [ ] UV and Workbench: no tab strip; the body is the UV form (Unwrap,
       Pack...) or the workbench.
4. [ ] Brush: Sculpt Tools, Paint Tools, Vertex Map Tools, Utilities.
5. [ ] Setup: Setup, Constraints, Inverse Kinematics, Weighting, then Rig
       (with the animation switch on). Buttons run; greyed ones say why.
6. [ ] Animation: no tabs, "No tools in this set yet" under the transform
       row.
7. [ ] Game: Vertex Map, Baking, Rig. Harden / Soften open their mode
       lists.
8. [ ] With UI build mode off, drag a tab (Items/Groups, or two views
       tabbed together) sideways in its row: it reorders. Drag it out of
       the row: nothing moves, no dock guide shows, and the message line
       says panels move in UI build mode.
9. [ ] Position/Rotation/Scale rows: a moved item shows no filled dot; a
       keyed channel still lights.
