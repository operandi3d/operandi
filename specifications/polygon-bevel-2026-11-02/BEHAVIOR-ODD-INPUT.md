# Odd input

**PB-O-01 Empty selection bevels every polygon.** Square with nothing
selected, Shift 0.3: the polygon is beveled (8 points, 5 polygons, cap at
z = 0.3). Selection after: nothing.

**PB-O-02 Negative Inset grows the cap outward.** Square, Inset -0.2: cap
corners at (-0.2,-0.2), (1.2,-0.2), (1.2,1.2), (-0.2,1.2).

**PB-O-03 Negative Shift moves the cap against the normal** (Square,
Shift -0.3: cap at z = -0.3; the square's normal is +Z).

**PB-O-04 Zero Shift and zero Inset still build the walls**: 8 points,
5 polygons, the new points coincide with the old ones.
