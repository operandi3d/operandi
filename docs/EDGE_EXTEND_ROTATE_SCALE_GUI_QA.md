# Edge Extend Rotate/Scale: GUI checklist

Spec: SPEC-EDGE-EXTEND-ROTATE-SCALE v1 (tutorial-gaps-2026-11-13). QA app,
fresh scene. Make a 2 m plane (Box with Y size 0 or a Plane primitive),
Edge mode, double-click its border to select the 4 edges, Action Center
Automatic. Press Z for Edge Extend.

- **EXQ-01 Handles.** Move arrows show at the selection's centre; no
  squares, rings or circles (spec v1.1). Tool Properties lists Offset,
  Rotate, Scale (100 %), Inset, Shift, Segments, Move/Rotate/Scale/Local
  Handles (Move ticked), Plane None, Haul Global Offsets, Make UVs U.
- **EXQ-02 Eye socket (EXT-RS-01).** Type Scale X/Y/Z 50 %: a 1 m square
  inside the plane, joined by 4 quads.
- **EXQ-03 Step down (EXT-RS-02).** Offset Y −1: the 1 m square drops 1 m,
  the handles go down with it.
- **EXQ-04 Turn (EXT-RS-03).** Rotate Y 45: the new square turns 45°.
- **EXQ-05 Shift+click (EXT-RS-04).** With Offset Y −1, Scale 50 %,
  Shift+click in the viewport: a second ring 1 m further down at half the
  size again; the values stay. Cmd+Z once: back to one ring, tool still on.
- **EXQ-06 Scale square drag.** Tick Scale Handles, drag a Scale square in: the new loop
  narrows on that axis only. Ctrl-drag: all three axes together.
- **EXQ-07 Rotate rings.** Tick Rotate Handles: three rings; drag one, the
  loop turns about that axis.
- **EXQ-08 Plane Scale.** Plane: Scale, then drag the circle that faces
  the view: the loop shrinks evenly in the view plane (the tutorial's move).
  Plane: Move: the circle moves the loop in that plane.
- **EXQ-09 Haul.** Drag off the handles: the loop moves in the view plane
  (Global Offsets). Haul: Local Offsets: drag right grows Inset (the loop
  steps outward along the surface), drag up grows Shift (along the normal).
- **EXQ-10 Local handles.** Tick Local Handles: a red Inset and a blue
  Shift arrow; each drags its value.
- **EXQ-11 Action centre.** Switch Action Center to Origin with the plane
  moved off the origin: the handles jump to the origin and Scale shrinks
  toward it.
- **EXQ-12 Make UVs.** By default (U) the new quads get Texture UVs with U
  running outward. Make UVs V, Drop, open the UV view on Texture: the
  new quads form a strip, V running outward.
- **EXQ-13 Drop, Undo, save.** Drop: the outer edges stay selected. Cmd+Z
  takes the extension(s) back one at a time. Save, reopen: geometry matches.
- **EXQ-14 Script.** `tool.attr edge.extend sclX 0.5` then `sclX ?`
  answers 0.5; `planeHandle scale`, `hauling local`, `uvs v` set the
  dropdowns.
