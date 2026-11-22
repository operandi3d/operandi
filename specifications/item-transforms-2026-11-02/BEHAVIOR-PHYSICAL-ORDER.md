# Item rotation order — v2 draft physical observation

Modo 17.1v1, 2026-11-03 UTC (2026-11-02 local). Jason changes the native
Properties dropdown; Codex reads exact local channels and evaluated world
matrices in separate bridge calls after preparation and after the change.
REA-assisted: no. One selected unparented unit-cube Mesh item, Items mode,
position zero, scale one, no active transform tool.

**IT-PHY-01 — Changing Order XYZ→ZXY keeps the measured Euler values and changes the item's orientation.**
Start with rotation (30,45,60) degrees and Order XYZ. Change only Properties
→ Transform → Order to ZXY. The three channels remain exactly (30,45,60)
degrees. Position and scale are unchanged; the local eight-point/six-quad
mesh is unchanged and the item stays selected. The evaluated world matrix
changes, corroborated by a visible orientation change. Therefore this UI
operation does not preserve the original orientation by recomputing Euler
angles in this measured case. IT-O-H3-rotation-order contains both complete
matrices and local meshes. Other order pairs, angles, parented items,
animated channels and undo remain unmeasured.

The earlier dropdown-opening failure is a historical desktop-input
diagnostic, superseded by this physical case; it is not a behavior rule.
