# Vertex snapping during Move

**SNP-VTX-01 — Component Vertex snapping lands the selected point exactly on the target.**
Fixture A contains separate unit quads at X=0…1 and X=3…4, Y=0…1, Z=0.
Select point 2, (1,1,0), in Vertex mode. Enable snapping; Component targets
Vertex only, no Global or Items targets. Selection Center Auto Axis;
Inner Range 24 px, Outer Range 40 px, Coordinate Rounding Fine.

Front view at 288.3 px/m (4 m spans 1153 px). Drag the center handle from
(821,444) to (1395,444), 3 px before the target projected at (1398,444).
That residual projects to about 0.010406 m. Point 2 lands exactly at (3,1,0),
X offset 2.000000 m. Repeat on a fresh fixture, releasing at (1383,444),
15 px short (0.052029 m): same exact landing. All other points and polygon
index lists remain unchanged. There are still 8 points / 2 polygons;
coincident points 2 and 7 are not welded, and only point 2 stays selected.
Oracles SNP-O-SN-03 and SNP-O-SN-03-15px.

These two releases establish successful landing within the configured range;
they do not establish the exact engage/highlight thresholds, path hysteresis
or whether a fixed pixel/world threshold governs other zooms. No held-drag
intermediate states were sampled. *(open: SNP-Q02)*

**SNP-STATE-01 — The observed session has distinct per-mode target lists.**
Before changes, snapping is off, only Global Vertex is enabled, and all
Component and Items targets are disabled. F11 exposes Inner Range 24 px,
Outer Range 40 px, Fine rounding, inactive Fixed Increment 10 mm and
Snap Angle 15.0°. These are session readings, not factory defaults. Target
flags and snapping-off state were restored and verified after the trials.
The ranges were read as integer pixels and angle to one displayed decimal.
