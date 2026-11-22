# Primitives — v2 draft additions

Modo 17.1v1, 2026-11-02. Numeric creation into empty meshes; reset then
listed overrides. Center at origin, Axis Y, face polygons, UVs on. No
symmetry, snapping or falloff. All output points, polygons and per-corner
Texture UVs are in the 21 new oracles. REA-assisted: no.

**PRIM-B3-01 — Quad Ball's level-zero cage scales linearly with Size but its measured half-side is larger than Size.**
Equal XYZ Size, Quad Ball, Level 0: eight points at every sign combination
of (a,a,a), six quads. The measured numbers are:

| Size | Cage half-side a | Full cage side | Center-to-corner distance |
|---|---|---|---|
| 0.5 | 0.654591 | 1.309182 | 1.133785 |
| 1 | 1.309182 | 2.618364 | 2.267570 |
| 2 | 2.618364 | 5.236728 | 4.535139 |

Thus Size is neither the raw cage half-side nor its corner radius: in
these samples half-side≈1.309182×Size, and corner distance≈2.267570×Size.
The primitive's displayed nominal bounds remain ±Size while the actual
level-zero points lie outside them. This answers the reproducible scaling
question without asserting an unmeasured subdivision-limit radius. The
user guide describes Quadball as intended for subdivision; a limit-surface
interpretation is not verified by these face-polygon cases. Exact points,
face order and UVs: PRIM-O-PR3-qball-size0.5/size1/size2. Other levels,
nonuniform sizes and the smoothed limit remain open.

**PRIM-B3-02 — Bulge 0.2 is accepted when Cube Radius is nonzero.**
Unit Cube, Segments X/Y/Z=2, Radius Segments 3, Sharp on. Radius 0.1 and
0.2 both accept Bulge 0.2 and produce 326 points / 344 polygons (304 quads
and 40 triangles). Their topology is identical, their coordinates differ,
and both remain within ±0.5 on every axis. Face centers remain at
(0,±0.5,0), (±0.5,0,0), (0,0,±0.5). First three points:

| Radius | Points 0,1,2 |
|---|---|
| 0.1 | (−0.4,−0.5,−0.4), (0,−0.5,−0.4), (0.4,−0.5,−0.4) |
| 0.2 | (−0.3,−0.5,−0.3), (0,−0.5,−0.3), (0.3,−0.5,−0.3) |

Every rounded/bulged point and UV is in
PRIM-O-PR3-cube-bulge-radius0.1/radius0.2. The earlier zero-radius refusal
remains valid. A general Bulge formula and a zero-Bulge comparison at these
radii were not measured in this pass.

**PRIM-B3-03 — Zero-height Cone ignores the tested Bottom Cap choices.**
Size X/Z=0.5, Size Y=0, Segments 1, Top Cap Single. For Sides 5 and 12,
each Bottom Cap None, Single, Quads or Radial yields exactly the same
respective point, polygon and UV lists: n perimeter points, one n-gon in
Y=0 facing +Y, corner order [0,1,…,n−1]. In particular None still creates
the disc, and Quads/Radial do not split it. No apex remains. Full results:
PRIM-O-PR3-cone-y0-sides5-none/single/quads/radial and
PRIM-O-PR3-cone-y0-sides12-none/single/quads/radial.

**PRIM-B3-04 — Negative-height Cone retains the selected Bottom Cap topology.**
Size X/Z=0.5, Size Y=−0.75 (full signed base-to-apex span −1.5), Segments
1, Top Cap Single. Base lies at Y=+0.75 and apex at Y=−0.75. The complete
polygon winding and UVs, including odd-cap triangulation, are pinned by
the eight PRIM-O-PR3-cone-y-0.75-sides5/12-<cap> oracles:

| Sides | Bottom Cap | Points / polygons | Face sizes |
|---|---|---|---|
| 5 | None | 6 / 5 | 5 triangles |
| 5 | Single | 6 / 6 | 5 triangles, one pentagon |
| 5 | Quads | 11 / 12 | 6 triangles, 6 quads |
| 5 | Radial | 7 / 10 | 10 triangles |
| 12 | None | 13 / 12 | 12 triangles |
| 12 | Single | 13 / 13 | 12 triangles, one 12-gon |
| 12 | Quads | 29 / 33 | 12 triangles, 21 quads |
| 12 | Radial | 14 / 24 | 24 triangles; pre-crash readback, see below |

**Stability qualification.** All requested readbacks were saved, then Modo
crashed at the end of the batch. The last operation was the negative-height,
12-sided Radial-bottom Cone; the exact triggering call was not isolated.
That case was not retried. Its oracle describes the saved pre-crash mesh,
not a stable workflow guarantee. Optional additional top-cap and zero-Bulge
controls were not executed. All cone cases here hold Top Cap Single;
independently varying the apex cap remains unmeasured.
