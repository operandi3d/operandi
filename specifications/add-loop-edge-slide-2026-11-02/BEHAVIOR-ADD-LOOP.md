# Add Loop: pointer placement and propagation

Fresh fixture per trial, Edge mode, no component selection, snapping off,
Fine coordinate rounding. G3 and irregular coordinates are defined in README;
every oracle includes the full input and output topology. Baseline: Middle,
Both Sides, Split, Absolute Distance and Preserve Curvature off, Gap zero;
Show Position on. Unless specified, Front view is 231 px/m (3 m spans 693 px),
world (0,0) projects near (763,934). Selection stays empty in every oracle.

**AL-01 — Clicking a quad-ring edge midpoint inserts a midpoint loop.**
G3 has 16 points / 9 quads. Click edge 1–2 at (1110,934), world about
(1.5,0,0). Position becomes 50%; four new points lie at X=1.5, Y=0,1,2,3,
Z=0. The three crossed quads split, giving 20 points / 12 quads. It ends
at the two mesh boundaries. Oracle AL-O-AL-01.

**AL-02 — Click position determines the fraction on the initial edge.**
On fresh G3, click (1052,934), about world X=1.25: Position 25%, new
loop X=1.250000. Click (1168,934), about X=1.75: Position 75%, new loop
X=1.750000. Each gives 20 points / 12 polygons. Oracles AL-O-AL-02a/b.
One pixel earlier, (1167,934), gives 74.5%, X=1.745000; this is a measured
sample, not a general rounding law (AL-O-AL-02b-quantization).

**AL-03 — A separate click followed by a drag changes the current loop's fraction.**
Click the midpoint first. Then drag (1110,934) → (1160,934), 50 px
(0.216450 m projected): Position becomes 71.5%, X=1.715000. A fresh
click then 100 px right (0.432900 m projected) gives 93.5%, X=1.935000.
Counts stay 20/12. Oracles AL-O-AL-03c/d. At a second zoom, 163 px/m
(3 m spans 489 px), click (1110,833) then drag 50 px right, projecting
to 0.306748 m: Position becomes 81%, X=1.810000 (AL-O-AL-03e).
The fraction change therefore depends on view scale. At the first zoom,
the samples give 0.0043/0.00435 fraction per pixel; at the second, 0.0062.
Intermediate held-drag preview was not sampled. Press-drag without a
separate initial click was inconclusive and is excluded. *(open: AL-Q02)*

**AL-04 — Split duplicates the cut's boundary points in the midpoint case.**
Same midpoint click with Split on gives 24 points / 12 polygons, rather
than 20/12: eight new points form two coincident four-point chains at X=1.5.
The cut sides use separate point indices. Caps off, Gap zero. Exact index
lists: AL-O-AL-04-split.

**AL-05 — The tested midpoint remains a midpoint with Middle or Both Sides enabled.**
Enable each separately on fresh G3 and repeat (1110,934). Both yield
Position 50%, 20/12 and the baseline coordinates. This does not establish
their effects away from the midpoint. AL-O-AL-04-middle/bothSides.

**AL-06 — A midpoint click replaces the pre-entered Position value.**
Set Position 25% before the initial click; click the midpoint. Readback is
50%, with the X=1.5 loop and 20/12 counts. AL-O-AL-04-position.

**AL-07 — Caps and Gap require Split in the tested setup.**
With Split off, requests for Caps on or Gap 0.1 m are refused. Actual Caps
stays off, Gap stays zero. The following midpoint click gives baseline
20/12 geometry. AL-O-AL-04-caps/gap record the actual settings; they are
not positive tests of Caps/Gap. Their geometry with Split on remains open.

**AL-08 — The tested ring stops at a triangle or pentagon and inserts the incoming edge point.**
Triangle variant replaces central quad [5,6,10,9] with [5,6,10] and
[5,10,9]. Midpoint bottom click changes 16/10 to 18/11: only the bottom
quad splits; the entering triangle becomes [5,16,6,10]. New points are
(1.5,1,0) and (1.5,0,0). AL-O-AL-05.

Pentagon variant adds point 16=(1,1.5,0) on the adjacent shared edge,
making the central polygon [5,6,10,9,16]. Counts 17/9 → 19/10. The
entering pentagon becomes a hexagon; the top quad is not cut. The same
two new coordinates appear. AL-O-AL-06. Neither case establishes behavior
for every triangle/n-gon arrangement.

**AL-09 — The five-quad pole fixture cuts the two adjacent quads without removing the pole.**
The oracle defines a five-quad radial fan about point 0=(1.5,1.5,0).
Click midpoint of edge 0–1 at (1225,588), world (2,1.5,0). Counts
11/5 → 14/7; new points are (2.309017,2.451057,0), (2,1.5,0),
(2.309017,0.548943,0). The center still has five incident edges; the
two traversed quads split toward the outer boundaries. AL-O-AL-07.

**AL-10 — A one-column open strip is split through all three quads.**
The strip covers X=1…2, Y=0…3. Midpoint bottom click (1110,934) adds
the four X=1.5 points, including both boundary endpoints: 8/3 → 12/6.
Oracle AL-O-AL-08.

**AL-11 — A second click without dropping Add Loop adds another loop.**
On G3 click (1110,934), then (879,934) on midpoint edge 0–1. The
X=1.5 loop remains and an X=0.5 loop appears: 16/9 → 20/12 → 24/15.
No tool drop between clicks. Oracle AL-O-AL-09.

**AL-12 — The tested unequal crossed edges use a common fraction, not a fixed distance.**
On G3-irregular, midpoint of bottom edge 1–2 is X=1.25; click (1052,934).
Position 50% gives X=1.25 on rows 0–2 and X=1.75 on the shifted top
row (AL-O-AL-10). Those crossed edges have equal widths, so a second
fixture moves point 10 to (2.5,2,0). The same click then gives X=1.25
at Y=0,1; X=1.5 at Y=2; X=1.75 at Y=3, all Z=0. The wider row's
half-width is 1 m versus 0.75 m below. Counts 20/12. This discriminates
equal fractions from equal distances. Oracle AL-O-AL-10b.

> Physical-session update: the modifier-trial blocked status below is superseded for the four measured trials by AL-PHY-01…04; broader modifier laws remain open.

Shift/Control/Command click trials remain inconclusive because physical
modifier delivery was not established; no modifier rule is asserted.
