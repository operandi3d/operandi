# Duplicate panel GUI pass (duplicate-panel-2026-11-20, part A)

One pass, after part B lands too. Item mode unless said.

1. Duplicate tab top to bottom: Extend (Radial Sweep, Bridge, Curve
   Extrude ▸), Clone (Clone, Curve Clone ▸, Scatter Clone ▸), Array (Array ▸,
   Radial Array ▸), Duplicate (Mirror), a divider, Tack Tool, Paste, then
   Commands: Duplicate · Duplicate Hierarchy.
2. Ctrl+click Curve Extrude: Bezier Extrude starts. Right-click or hold a
   flyout: the list opens.
3. Three cubes apart, all selected. Array ▸ Transform Array, click in the
   view: the cubes line up on the grid, no new items. Right-drag Count: the
   extra cubes stay where they were. Space drops; Undo puts them back.
4. Radial Array ▸ Transform Radial Array, click: the cubes go round the
   axis and each turns to face round.
5. Scatter Clone ▸ Transform Scatter, click: cubes scatter; Seed reshuffles.
6. Curve Clone ▸ Transform Pen, click three points: one cube per point.
   Bezier Transform: cubes follow the curve.
7. Lock one cube (Item Properties ▸ Lock): it stays put and the panel says
   "1 locked item stays where it is."
8. Parent a sphere under a cube, select the cube, Duplicate Hierarchy: a
   copy of both, the sphere copy under the cube copy. Plain Duplicate copies
   the cube alone.
9. Bridge, Tack Tool and Paste on this tab start the same tools as on the
   Polygon / Topology / Create tabs.
