# Shared pivot service: GUI check

Rulebook: specifications/ac-symmetry-rulebook-2026-11-19 (ACS-xx).
Set up once: a cube with 4 segments in X (Box tool, Segments X 4), Symmetry X.

1. **Click picks the side (ACS-07).** Polygon mode, select the two end
   faces (+X and −X). Move (W): the handle is on the +X face. Click empty
   space on the −X side: the handle jumps there. Type Move X 0.25: the −X
   face moves +0.25, the +X face −0.25. Undo, repeat with a click on +X:
   the reverse.
2. **Extrude sideways (ACS-21).** Same selection, Extrude, drag the
   handle sideways along X: the two faces move opposite ways in X (mirror
   images), the result stays symmetric.
3. **Linear Falloff + Symmetry (ACS-13).** Falloff Linear, start on the
   −X side, end on the +X side, running across the cube. Select a mirrored
   pair of top faces, Move up: both sides fade the same way, as mirror
   images. The handle is on the + side.
4. **Selection Border + Symmetry (ACS-01).** Action Center Selection
   Border; select the whole top row of faces. Rotate (E): the handle sits
   on the + half's border, not on the plane.
5. **Smooth Shift scales about the Action Center (ACS-20).** Action
   Center Automatic, select the two end faces, Smooth Shift, Scale 0.5:
   each face shrinks about its own centre (not the item origin); the
   result stays symmetric. Action Center Origin: both shrink toward the
   origin instead.
6. **Parent and Pivot.** Action Center menu: Parent, Pivot and Pivot
   Center Parent Axis are available (no longer greyed). Pivot on a moved
   cube: the handle sits at the item's position.
