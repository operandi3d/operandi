# Falloff mechanism

**FAL-M-01** At most one falloff is active. Activating a second type turns the
first off. (Linear activated while Radial was active: Radial reads off,
Linear on.)

**FAL-M-02** A falloff stays active across transform applications until it is
turned off; while active it scales every point's share of the transform by
its weight w (0…1). With w = 1 the point gets the full transform, with w = 0
none.

**FAL-M-03** A falloff's settings are kept between uses: on reactivation
Linear and Radial showed the values last set, not reset values. *(Factory
values: FAL-Q07.)* Observed starting values of Custom In/Out: In 0, Out 1.

**FAL-M-04** Auto Size for Linear, per axis A (X, Y or Z), uses the bounding
box of the selection: Start and End are the box's minimum and maximum along
A; the other two coordinates of both are the box center. Setup: 7×7-point
grid (x, y from -3 to 3), polygons covering x -1…2, y -1…1 selected. X →
Start (-1,0,0), End (2,0,0). Y → Start (0.5,-1,0), End (0.5,1,0). Z (box has
no depth) → no change.

**FAL-M-05** Reverse swaps Start and End: (0.5,-1,0)/(0.5,1,0) becomes
(0.5,1,0)/(0.5,-1,0).

**FAL-M-06** Auto Size for Radial sets Center to the selection's box center
and Size to half the box extents: (0.5, 0, 0) and (1.5, 1, 0) for the
selection above. With nothing selected the whole mesh is used: center
(0,0,0), size (3,3,0).
