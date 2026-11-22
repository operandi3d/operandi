# Twist Bevel GUI check

Spec: specifications/twist-bevel-2026-11-15. Use a 1 m cube and a 4×1 row
of plane polygons; do steps 1–4 on a SubD copy of the cube too.

1. **TB-G01 Start and size.** Polygon mode, select the cube's top polygon,
   click Twist Bevel on the Polygon tab (under Bevel). Nothing changes yet.
   Drag up anywhere off the handles: a straight tube of 4 rings grows; the
   top stays selected. Drop, then Cmd+Z undoes it in one step.
2. **TB-G02 Handles.** Start again and type Shift 1. Drag each handle:
   blue Shift arrow at the cap (length), red Inset square (out grows the
   cap), blue ring round the cap (twist), green arrow at the base (curls the
   tube toward it), green ring round the base (turns which way it curls).
   The status line shows the value while you drag; the cap handles follow
   the cap as it bends.
3. **TB-G03 Panel.** Set Rotation 90°, Bend 90°, Profile −0.5, then Profile
   Bias 1, Segment Spacing 1, Center Bias −1: each changes the tube at once.
   Profile Bias is greyed while Profile is 0; Profile and the spacing rows
   are greyed at Segments 1.
4. **TB-G04 Continue.** With a bent tube, Shift+click off the handles, then
   drag up: a second twist bevel grows from the cap with the same curl.
   Drop; Cmd+Z twice undoes each one.
5. **TB-G05 Group.** On the plane row select two neighbours: one tube with
   Group Polygons on, two with it off.
6. **TB-G06 Falloff and Symmetry.** Linear falloff along the row, all four
   selected, Group off: tubes grow from short to long. Symmetry X with the
   outer two selected: mirror-image tubes (curl and twist mirrored).
7. **TB-G07 Snapping.** Snapping on (vertex targets): drag the Shift arrow;
   the cap centre lands on a vertex's height along the spine.
8. **TB-G08 Credits.** Help ▸ About Operandi shows the Twist Bevel credit
   line (FUSION; Modest by Christian).
