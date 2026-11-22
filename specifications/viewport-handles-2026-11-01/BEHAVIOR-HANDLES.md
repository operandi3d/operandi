# Transform handles: measured pointer cases

These requirements cover the recorded fixture and view only. Fresh unit cube
at the origin; point order and polygon lists are in every oracle. Select the
+Z face (polygon 1, points 4–7) in Polygon mode. No named Action Center preset;
snapping off. Topology remains 8 points / 6 polygons, polygon 1 stays selected,
and the four other points remain unchanged in every case below.

**HND-MOVE-01 — The Move X arrow translates the selected face along X.**
At 624 px/m (±1), drag (1695,614) → (1795,614): 100 px, projecting to
0.160256 m. The four face points gain exactly 0.160000 m in X. A fresh
50 px drag from the same start gives 0.080000 m (0.080128 m projected).
Oracles HND-O-MOVE-01 and HND-O-MOVE-02.

**HND-MOVE-02 — The same 100 px Move drag covers more world distance after zooming out.**
At 442 px/m, drag (1550,606) → (1650,606), projecting to 0.226244 m.
The measured X offset is 0.225000 m. The 624 px/m trial gave 0.160000 m.
Thus the offset depends on view scale; these samples do not define rounding
at every pixel. Fine rounding was later observed in the unchanged session.
Oracle HND-O-MOVE-04. *(open: HND-Q05)*

**HND-ROTATE-01 — The visible blue rotation ring rotates the selected face about Z.**
At 442 px/m, the ring center is (1458,606), radius about 120 px
(0.271493 m projected). A straight chord from (1578,606) to (1458,486)
gives +89.500000°, not exactly 90°. From the same start to (1542,522)
gives +45.000000°. The selected face rotates about (0,0,0.5); all exact
coordinates are in HND-O-ROTATE-01 and HND-O-ROTATE-02. The trials do not
establish a linear angle-per-chord-length law.

**HND-SCALE-01 — The X scale endpoint changes X scale linearly across the two tested lengths.**
At 442 px/m, drag (1572,606) → (1672,606): scale (1.690000,1,1).
A fresh 50 px drag to (1622,606) gives (1.345000,1,1). Scale change is
0.006900 per pixel across these samples. The pointer distances project to
0.226244 m and 0.113122 m. Negative Scale is on; Absolute Handle is off.
The selected points' X coordinates become ±0.845000 or ±0.672500.
Oracles HND-O-SCALE-01/02. Scale at a second zoom remains unmeasured.

**HND-TRANSFORM-01 — The combined Transform X arrow translates the selected face.**
At 442 px/m, (1538,606) → (1638,606) gives X +0.225000 m;
a fresh 50 px drag to (1588,606) gives +0.115000 m. Projected pointer
distances are 0.226244 m and 0.113122 m. Other translation and rotation
values stay zero; scales stay one. Oracles HND-O-TRANSFORM-01/02.

Off-handle diagonal movement and a click-only trial are retained as raw
inconclusive records. They do not establish an off-handle rule. Middle/right
drags, the full handle inventory, modifiers and other tools remain open.
