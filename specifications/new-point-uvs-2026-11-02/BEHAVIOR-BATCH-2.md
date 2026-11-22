# New-point UVs — v2 draft additions

Observed by scripted probes in Modo 17.1v1 on 2026-11-02 (Codex), exact
six-decimal mesh and polygon-corner UV readback. REA-assisted: no. G3 and
its Texture map are defined in README. Fresh fixture; no symmetry, falloff
or snapping. Every oracle includes input and output UVs at every corner.

**UVN-B2-01 — Edge Bevel interpolates the edited non-affine UV along source edges in the tested corners.**
G3, edge 5–9, Inset 0.2. Change only point 5's continuous UV from
(1/3,1/3) to (0.5,0.5), preserving all other input UVs. Counts stay
20 points / 12 polygons as in the affine control. At new point (0.8,1,0),
UV is (0.4,0.466667); at (1.2,1,0), (0.533333,0.466667); at
(1,0.8,0), (0.466667,0.4). These match 20% interpolation from point 5
toward the corresponding original edge neighbor. They differ from the
old world-affine map evaluation (X/3,Y/3). Upper-end corners whose source
UVs did not change retain their affine values. The new strip's ordered
UVs are [(0.266667,0.666667),(0.4,0.466667),(0.533333,0.466667),
(0.4,0.666667)]. UVN-O-UV2-01-affine/nonaffine. General interpolation
inside arbitrary curved polygons remains open; these are edge samples.

**UVN-B2-02 — The tested Edge Bevel strip uses the right neighbor's UV frame even when the shifted seam frame is on the left.**
G3 polygons 0,3,6 (left column) carry U+1 per corner; continuous values
remain (X/3,Y/3). Bevel edge 5–9 at X=1 by 0.2. Strip polygon 9's
four UVs are [(0.266667,0.666667),(0.266667,0.333333),
(0.4,0.333333),(0.4,0.666667)], all unshifted, including corners on
the geometric left. The right-seam control, polygons 2,5,8 shifted U+1,
edge 6–10 at X=2, gives strip U values 1.6/1.733333, all shifted.
Thus seam-frame magnitude alone does not choose the frame; in these
G3 orientations it is the right neighbor's frame in both cases. Counts
20/12; exact end-triangle and existing-polygon UVs/selections in
UVN-O-UV2-03-leftseam/rightseam-control. Frame choice under reversed
corner ordering and other geometry remains open.

**UVN-B2-03 — Edge Chamfer's end mode changes the zero-area triangles in the tested planar case.**
G3 continuous Texture map, edge 5–9, Offset 0.2, one segment. Both Stop At
Edge states give the same result for each end mode:

| Edge End | Points / polygons | End triangles |
|---|---|---|
| Profile | 20 / 14 | Four zero-area triangles: two coincident copies at each end |
| Triangle Fan | 20 / 12 | Two nonzero-area triangles, one at each end |

Profile's bottom duplicates both use [5,18,17], with corners on Y=1;
top duplicates lie on Y=2. Triangle Fan extends its end tips to
(1,0.8,0) and (1,2.2,0), with UVs (1/3,0.266667) and
(1/3,0.733333); strip UVs remain affine. Full topology, UVs and empty
selection: UVN-O-UV2-04-stop0/stop1-profile/triangleFan. This extends
UVN-T-02's measured default Profile result; it does not generalize the
coincident triangles to all end modes.
