# Transforms with Symmetry on

All cases: numeric application of the transform tool (no pointer input),
action center Origin unless stated, falloff and snapping off.

**SYM-TR-01** Only selected elements move. An unselected counterpart is not
edited. Setup: F1, Symmetry X, only point 9 selected. Action: Move
(0.25, 0.1, 0). Result: point 9 goes to (1.75, 1.1, 0); point 5 stays at
(-2, 1, 0); selection after is still only point 9. (SYM-O-01.)

> v2 draft: applying this side rule unchanged to a displaced Work Plane is
> superseded by the measured example in SYM-B2-08. The original zero-plane
> examples below remain valid; general displaced-plane composition is open.

**SYM-TR-02** A selected point on the **negative** side receives the
transform as entered; a selected point on the **positive** side receives the
transform mirrored across the plane. For Move with axis X that means the X
component is negated for positive-side points; Y and Z are unchanged.
Setup: F1, Symmetry X. Results: point 5 alone, Move (0.25,0.1,0) →
(-1.75, 1.1, 0). Point 9 alone → (1.75, 1.1, 0). Point 8 (x = 1) alone,
Move (0.25,0.1,0.3) → (0.75, 1.1, 0.3). With axis Y, point 9 (y = 1, positive)
and Move (0.25,0.1,0) → (2.25, 0.9, 0).

**SYM-TR-03** Therefore a selected mirrored pair stays mirrored. Setup: F1,
Symmetry X, points 5 and 9 selected, Move (0.25,0.1,0). Result: point 5 at
(-1.75,1.1,0), point 9 at (1.75,1.1,0). The move is applied once to each,
never twice. (SYM-O-02.)

**SYM-TR-04** A point exactly on the plane does not move across it: the
component along the axis is dropped and the other components apply. Setup:
F1, Symmetry X, point 7 (0,1,0) selected. Move (0.25,0,0) → stays (0,1,0).
Move (0,0.1,0.2) → (0, 1.1, 0.2). (SYM-O-03.)

**SYM-TR-05** Rotate and Scale follow the same rule: positive-side points
get the mirror image of the transform. Setup: F1, Symmetry X, action center
Origin, points 9, 5, 8, 6 selected, Rotate 30° about Y. Results: point 5
(-2,1,0) → (-1.732051, 1, 1); point 6 → (-0.866025, 1, 0.5); point 8 →
(0.866025, 1, 0.5); point 9 → (1.732051, 1, 1). With Symmetry off the
positive-side points instead go to z = -0.5 and -1. Then Scale X 0.5, Y 2
on points 9 and 5 gives (-0.866025, 2, 1) and (0.866025, 2, 1).
(SYM-O-05, SYM-O-06.)

**SYM-TR-06** With an Offset, sides are measured from the offset plane.
Setup: F1, Symmetry X, Offset 0.5. Point 9 (x = 2, positive) moved
(0.25,0,0) → x = 1.75; point 6 (x = -1, negative) → x = -0.75. (SYM-O-10.)

> v2 draft: The unobserved status below is superseded for the two tested operations by SYM-B2-11.

**SYM-TR-07** Undo: not observed (SYM-Q07).
