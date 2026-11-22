# Brief: run the Transform pipe on UVs (stage 1)

Received 2026-11-16 (from research, via the project thread).

Goal: every tool built on the Transform pipe edits UVs when it's used in a UV
view. Don't build a separate set of UV tools: make one change to the pipe, and
Move, Rotate, Scale, Transform, Element Move, Soft Drag, Soft Move and the Warp
presets (Smooth, Jitter, Shear, Taper, Pole) all work on UVs. Cite rule numbers
in commits.

1. When the tool's view is a UV view, the pipe's "points" are the current UV
   map's UV positions of the visible polygons, as (U, V, 0). The tool writes
   back U and V only. 3D positions and other maps never change (UVT-06).
2. A 3D point with two UV positions (a seam) is two separate pipe points. A UV
   position moves only with its own polygons (UVT-42: a UV position used only
   by selected polygons moves; one shared with an unselected polygon across a
   seam doesn't).
3. Picking and highlighting under the pointer (Element Move, element falloff,
   element action centre) use the UV view's drawn UV elements.
4. Falloffs measure in U and V. The screen falloff stays a screen circle.
   "Connected" means joined in UV, i.e. the same island (UVT-04: a group of
   polygons joined through shared UV positions, not just shared 3D points).
5. Handles are flat: a U arrow (teal), a V arrow (yellow), a centre square and
   one rotate ring about the view axis. The X/Y fields edit U/V, and the Z
   fields are greyed.
6. Symmetry in a UV view is UV symmetry (a U or V line with an offset). That
   isn't built yet, so symmetry is off in UV views. Never mirror UVs with 3D
   symmetry.
7. If the mesh has no UV map, the tool does nothing in the UV view.
8. Undo: one step per tool session, as in 3D.

Scope (UVT-02): with a selection, act on that selection's UVs. With nothing
selected, act on all visible polygons. Hidden polygons are never touched.

Minimum tests:
- Move U +0.25 on one selected UV polygon of a cube's map: only that polygon's
  UVs change, its seam neighbour's UVs stay put, and 3D points are unchanged.
- Element Move, Range 0, dragging a UV point: only that point moves.
- Element Move with Use Connectivity, Range 1.0, on an island next to another:
  the other island doesn't move.
- Soft Drag in a UV view: points inside the screen circle move by the falloff
  weight, and points outside don't.
- Rotate 90° about the selection centre on (0,0) (1,0) (1,1) (0,1): the corners
  go round one place counter-clockwise (build guess).
- Regression: the same tools in a 3D view behave exactly as before.

Out of scope: sculpt tools on UVs, Box Transform, UV symmetry, Tear Off
(UVT-41).

Guesses to keep easy to change, each behind ONE named constant or setting with
a comment: UVT-Q11 (Range units in a UV view: UV units, versus screen pixels),
UVT-Q12 (Use Connectivity means the UV island, versus 3D connectivity), and the
rotate direction.

Order: (1) the pipe core in UV space + Move/Rotate/Scale/Transform, (2) picking
+ Element Move + element falloff/centre, (3) the other falloffs + Soft
Drag/Soft Move, (4) the Warp presets, (5) Slide on UVs if cheap.
