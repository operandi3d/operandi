# Action Center: Automatic and Local

**ACT-AUTO-01 — Automatic places the center at the unselected cube's center.**
Use a unit cube centered at (2,1,0) in a mesh at identity, Polygon mode,
no component selection. Activate Automatic then Rotate through the UI.
Tool Properties shows center (2,1,0) m and Auto Axis Z. Enter Rotation Y
90° while the tool is live. Readback confirms rotation about X=2, Z=0;
the UI supplies the Y center. All 8 points / 6 polygons remain and selection
stays empty. Exact points: ACT-O-AC-03.

**ACT-AUTO-02 — Automatic uses the bounding-box center for the tested asymmetric unselected mesh.**
Use the preceding cube, but move its (+X,+Y,+Z) corner from (2.5,1.5,0.5)
to (3.5,1.5,0.5). Bounds center is (2.5,1,0); vertex average is (2.125,1,0).
No components selected. Automatic + Scale shows center (2.5,1,0) m.
Enter X, Y and Z Scale 0% through the live Tool Properties fields. All eight
points become exactly (2.5,1,0); the six polygon index lists and empty
selection remain. This separates the bounding-box center from the vertex
average for this fixture. Oracle ACT-O-AC-03-asym. Hidden points, nonuniform
items and selected subsets remain untested. *(open: ACT-Q02)*

**ACT-LOCAL-01 — Local rotates the two disconnected quads about separate centers and normals.**
Select both polygons of fixture I: quad 0 in XY from (0,0,0) to (1,1,0),
quad 1 in YZ at X=5, Y=0…1, Z=−0.5…0.5. Activate Local and Rotate,
Gimbal on. Front view, 230 px/m; the first center is about (648,588), ring
radius 120 px (0.521739 m projected). Drag a straight chord from (768,588)
to (648,468), giving +90.5° in the tool. Quad 0 rotates about (0.5,0.5,0)
and its Z normal; quad 1 about (5,0.5,0) and its X normal. For example,
point 0 becomes (1.004344,0.004382,0), and point 4 becomes
(5,1.004344,−0.495618). Counts stay 8/2; polygons 0 and 1 stay selected.
All exact coordinates: ACT-O-AC-10. Shared-point membership and Local Move
are still open. Numeric rotation fields were absent in this Local tool UI;
this requirement comes from the ring gesture.

No automatic-axis switching threshold, general placement/haul rule or other
center behavior is asserted. AC-02 crashed during setup and was skipped.
