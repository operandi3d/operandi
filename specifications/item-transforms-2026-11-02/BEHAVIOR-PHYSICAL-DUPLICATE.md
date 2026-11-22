# Item drag and duplication — v3 draft physical observations

Modo 17.1v1, 2026-11-03 UTC (2026-11-02 local). Jason performs physical
drags; Codex reads exact item channels, evaluated matrices, local
geometry and source relationships. Native UI confirms the selected copy is
a Mesh Instance with 100% scale on all axes. REA-assisted: no.

Each trial starts with a fresh unparented unit-cube Mesh, position (1,0,0) m,
rotation (0,45,0) degrees, Order ZXY, scale (1,1,1), Items mode, Move active,
Front orthographic, snapping off and no named Action Center. Duplicate Mode
is retained at Instance. Calibration is approximately 844 px/m vertically.
The actual physical drag paths were not measured, so their displacements
are individual results and cannot establish per-pixel sensitivity.

**IT-PHY-02 — An ordinary left drag translates the existing item.**
Dragging the red X handle right leaves one Mesh at (1.03,0,0) m. It remains
selected; rotation, scale and local geometry stay unchanged.
Oracle IT-O-H5-left-control.

**IT-PHY-03 — The tested middle-button drag creates no second item.**
The fresh source ends at (1.06,0,0) m and remains the only Mesh and selected
item, with unchanged rotation, scale and local geometry.
Oracle IT-O-H5-middle-clone. Input Editor labels the middle-button event
Transform: Clone. The Move and Transform user-guide sections describe that
interaction as repeating an offset; this trial does not establish a repeat
formula because the pointer path and prior offset were not controlled.
The event label alone does not establish object duplication.

**IT-PHY-04 — Control+Shift left drag creates a linked instance.**
Input Editor's Transform Tool actions show the Duplicate action on
Control+Shift+left click-and-drag. With these keys held, a short rightward
drag of the red X handle creates a Mesh Instance at (1.084,0,0) m, linked to
the original source. Only the instance is selected. The original stays at
(1,0,0) m with exactly unchanged local channels, world matrix and mesh.
The instance retains rotation (0,45,0), Order ZXY and scale (1,1,1).
The overlapping pair can look wider, although neither item is scaled.
Oracle IT-O-H5-control-shift-duplicate contains both evaluated matrices.

These three oracles include complete source geometry and before/after
channels. Other Duplicate Mode values, other axes, parented items, modifier
changes during a drag, exact drag sensitivity and Undo remain unmeasured.
Move's captured independent preferences were restored with zero mismatches;
the tool was dropped without Reset or Apply.
