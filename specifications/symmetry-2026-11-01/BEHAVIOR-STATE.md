# Symmetry state

**SYM-ST-01** Symmetry is one setting with the values Off, X, Y and Z
(plus an Arbitrary axis given by a vector). In the observed session it was
Off with the axis set to X. *(Observed; whether Off/X is the factory default
is open: SYM-Q01.)*

**SYM-ST-02** With axis X the mirror plane is the plane x = Offset in the
**mesh item's own coordinates**. Setup: F1 inside an item translated by
(3,0,0), Symmetry X, Offset 0. Action: select point 9 (local (2,1,0)) and
run Select Symmetrical. Result: point 5 (local (-2,1,0)) is added, i.e. the
plane is local x = 0 (world x = 3), not world x = 0. (Oracle SYM-O-09.)

**SYM-ST-03** Offset moves the plane along its axis. Setup: F1, Symmetry X,
Offset 0.5. Result: point 9 (x = 2) pairs with point 6 (x = -1) and point 8
(x = 1) pairs with point 7 (x = 0), so the plane is x = 0.5. (SYM-O-10.)

**SYM-ST-04** Axis Y uses the plane y = 0 and axis Z the plane z = 0. Setup:
a 3×3-point grid spanning -1…1; point 0 at (-1,-1,0) for Y (grid in XY) and
(-1,0,-1) for Z (grid in XZ). Select Symmetrical adds point 6, at (-1,1,0)
and (-1,0,1) respectively.

**SYM-ST-05** Every point has a **side**: positive when its coordinate along
the axis is greater than the plane, negative when smaller, and on the plane
when equal. The side decides how transforms apply (see
BEHAVIOR-TRANSFORMS.md).
