# Mesh commands — v2 draft additions

Observed in Modo 17.1v1 on 2026-11-02. Numeric operations; all input
and output points, polygon winding and component selections are preserved
in the new oracles. REA-assisted: no. No symmetry, snapping or falloff.
These additions preserve earlier requirement and oracle IDs.

**CMD-B3-01 — Subdivide Count specifies the tested number of intervals, with a minimum of two.**
Two vertically stacked quads share the edge (0,0,0)–(1,0,0), selected alone.
At default Percent, Counts 1 and 2 both insert (0.5,0,0); Count 3 inserts
(0.333333,0,0) and (0.666667,0,0); Count 4 inserts (0.25,0,0),
(0.5,0,0), (0.75,0,0). Both adjacent faces gain the inserted points without
being split into additional faces. Count 1 with Percent 10/25/50/75 gives
exactly (0.1/0.25/0.5/0.75,0,0), one point per case. The previous
Percent 25 → 99.9% record was an out-of-range driving value and does not
specify the UI's 25% behavior; its numeric observation is retained.
CMD-O-M3-01-count1/count2/count3/count4 and percent10/percent25/percent50/percent75.

**CMD-B3-02 — In the measured quad neighborhoods, Vertex Collapse merges the selected point and its directly edge-connected neighbors.**
The replacement is their arithmetic mean, including the selected point.
Incident quads collapse away; outside quads reference the replacement.
Two-ring quad fixtures with center valences 3/4/5 start with 13/17/21
points and 9/12/15 faces and finish with 10/13/16 points and 6/8/10
faces. The first ring's alternating diagonal corners survive; the center
and its 3/4/5 edge neighbors become one point at (0,0,0). An asymmetric
valence-3 control moves only the selected center to (0.2,0.1,0): the
replacement is (0.05,0.025,0), distinguishing averaging from keeping the
selected position. G3 border point 1 has neighbors 0,2,5: these four
points become (1,0.25,0); the two incident quads disappear, yielding
13 points / 7 faces. Exact surviving connectivity and renumbering are
pinned by CMD-O-M3-02-valence3/valence4/valence5/asymmetric3/border.
This rule is measured for these quad neighborhoods; arbitrary mixed-face
or overlapping selections remain open.

**CMD-B3-03 — Quadruple still makes no geometric or winding change in the requested triangle sets.**
Two triangles forming a non-square convex quad, consistently wound or
with one triangle reversed, remain 4 points / 2 triangles. A 2×2 grid
triangulated into eight faces, consistently wound or with triangles 1
and 4 reversed, remains 9 points / 8 triangles. All selected polygons
remain selected. These observations do not establish a positive case or
a general activation rule. CMD-O-M3-03-pair-same/pair-mixed/grid-same/grid-mixed.

**CMD-B3-04 — Align repairs a flipped middle or end polygon in a connected strip.**
Four XY quads along X. Reverse polygon 1, then Align all four: all four
windings become +Z. Repeat with only polygon 3 reversed: again all become
+Z. Points, counts and selected polygon set stay the same; cyclic corner
starting positions can change. The earlier assertion that the mixed
winding example made no change is superseded: the earlier oracle itself
already contains a corrected connected face. Its disconnected reversed
island remains reversed. CMD-O-M3-04-flipped1/flipped3 give the complete
corner orders. Which orientation wins for arbitrary connected components
or different seed selections remains open.

**CMD-B3-05 — Edge Split acts on the closed four-edge border of G3's middle quad.**
Select edges 5–6,6–10,10–9,9–5. Gap zero duplicates four points:
16/9 → 20/9 with Caps off, or 20/11 with Caps on. The four duplicate
coordinates coincide with the original loop corners. Gap 0.1 instead
produces 23 points, with 9 faces (Caps off) or 11 (Caps on). The middle
quad's corners move to (1.05,1.05), (1.95,1.05), (1.95,1.95),
(1.05,1.95); the outside corner treatment is asymmetric and must follow
the full measured coordinates/connectivity rather than a guessed uniform
offset. Caps adds two polygon faces, including a reversed face over the
original middle-quad coordinates. Exact points, corner order and selection:
CMD-O-M3-05-caps0-gap0/caps1-gap0/caps0-gap0.1/caps1-gap0.1.
A single interior edge's earlier no-change result remains valid for that
fixture; it does not extend to this closed loop.
