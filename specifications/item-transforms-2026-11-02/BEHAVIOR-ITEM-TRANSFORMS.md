# Item transforms through the live tools

Each trial starts fresh. Main mesh: local unit cube centered at origin,
item position (1,0,0) m, Euler rotation (0,45,0)°, scale (1,1,1), order ZXY.
Second cube is a child of a locator: child local position/rotation zero,
scale one; locator position (3,0,0) m, rotation (30,0,0)°, scale one.
Both use ZXY. Items mode, main selected unless specified, no named Action
Center preset, snapping off. Values below are local item channels, not
world-matrix decompositions. Non-target items remain unchanged in each oracle.

**IT-01 — Typing Move's X offset translates the selected item.**
Activate Move, type 1 m in the live tool's Position X field (initially zero).
Main position becomes (2,0,0), rotation (0,45,0), scale one, order ZXY.
The item Properties panel's absolute X was initially 1 m; the tool field
is an offset in this case. Oracle IT-O-IT-01.

**IT-02 — The X Move handle and combined Transform translation handle move the item.**
Front orthographic, 359 px/m: main center about (787,588), world origin
about (428,588). Move drag (880,588) → (980,588), 100 px (0.278552 m
projected), gives tool X offset 0.280000 m and item position (1.28,0,0).
A fresh Transform drag (865,588) → (965,588) gives the same result.
Rotation/scale/order unchanged. Oracles IT-O-IT-02/07. Second zoom and
other combined handles remain open. *(open: PAR-Q05)*

**IT-03 — Gimbal-on X rotation adds to the tested item's X Euler channel.**
With Rotate and Gimbal on, enter Rotation X 30°. Main channels become
(30,45,0)°, order ZXY; position (1,0,0), scale one. Oracle IT-O-IT-03.
This measures the stated order/Gimbal configuration only; other composition
modes and action axes remain open.

**IT-04 — Negative Scale permits a negative percentage scale.**
Scale, Negative Scale on: type −100% into Scale X. Main scale becomes
(−1,1,1); position/rotation/order unchanged. IT-O-IT-04.

**IT-05 — With Negative Scale off, the tested negative entry leaves the item scale unchanged.**
Disable Negative Scale, then type −100% into Scale X. Tool X reads zero,
but the item's scale remains (1,1,1); all other channels unchanged. No
message was observed. This is the measured item result, and does not imply
that zero scale was applied. Oracle IT-O-IT-05.

**IT-06 — A positive percentage scales the selected item.**
Negative Scale on, type 50% into Scale X: scale becomes (0.5,1,1), with
other channels unchanged. Oracle IT-O-IT-06.

**IT-07 — Move's Y entry changes the selected child's local position.**
Select child only. Enter Move Position Y 1 m. Child local position becomes
(0,1,0), rotation zero, scale one, order ZXY. Rotated parent and main item
are unchanged. This establishes local channel output; world displacement
was not read back. Oracle IT-O-IT-08.

**IT-08 — The item X scale endpoint changes scale in the recorded view.**
At 359 px/m, drag (897,588) → (997,588), 100 px or 0.278552 m projected.
Negative Scale on. Main scale becomes (1.69,1,1); position and rotation
unchanged. Oracle IT-O-IT-10. No scale-per-pixel invariance across zooms
is established.

**IT-09 — The red X ring changes the item's X Euler channel with Gimbal on.**
Front view at 359 px/m. Ring center about (787,588), vertical radius
120 px (0.334262 m projected), horizontal radius about 85 px because
the item is turned 45° about Y. Drag chord (872,588) → (787,468).
Main rotation becomes (−90.5,45,0)°, order ZXY; position/scale unchanged.
Oracle IT-O-IT-11. This is a measured chord result, not an exact 90° gesture.

> Physical-session update: the concluding rotation-order blocked status is superseded for XYZ→ZXY by IT-PHY-01. Earlier numeric examples and IDs are unchanged.

Rotation-order dropdown attempts did not expose a menu, so XYZ→ZXY
orientation preservation remains unknown. Clone/haul, other action centers
and other axes were not run.
