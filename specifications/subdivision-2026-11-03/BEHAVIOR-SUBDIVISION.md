# Subdivision surfaces

Observed in Modo 17.1v1 on 2026-11-03 by scripted probes: build a cage,
convert its polygons, set the level, then freeze the smooth surface into
plain polygons and read every point back (six decimals). Option names and
intent from the user guide's Mesh Item page; nothing copied. REA-assisted:
no.

Modo has two kinds of subdivision polygon, chosen per polygon:
**Subdivision** (Modo's own scheme, "SubD", Tab) and **Catmull-Clark**
(Pixar-style, Shift+Tab). A plain polygon is **Face**. One mesh can mix all
three.

## Switching

**SDV-01 — Tab and Shift+Tab toggle per polygon.** Tab turns each Face
polygon into Subdivision, and each Subdivision **or** Catmull-Clark polygon
back into Face. Shift+Tab turns each Face polygon into Catmull-Clark, and
each Subdivision or Catmull-Clark polygon back into Face. So switching
between the two smooth kinds takes two presses (smooth → Face → other
smooth). A mixed set flips polygon by polygon: on a cube with one
Catmull-Clark, one Subdivision and four Face polygons, Tab gives Face, Face
and four Subdivision. (SDV-O-toggle.)

**SDV-02 — The toggle acts on the selected polygons, or on every polygon
when none is selected.** Points, polygons and their order are unchanged;
only the type changes. (SDV-O-toggle.) Shift+Tab's binding is this toggle
directly; Tab's binding first gives a viewport value field priority when one
is open, and otherwise toggles. *(open: SDV-Q05 for Tab inside other
contexts.)*

## Settings (per mesh item)

**SDV-03 — Each mesh has separate levels for each kind, and for display and
render.** Defaults on a new mesh: Subdivision level 2, Catmull-Clark level
2, Subdivision render level 2, Catmull-Clark render level 2, Boundary Rule
Crease, UV Boundary Rule Crease, Linear UVs off. The display level is what
the viewport shows and what Freeze produces; the render levels apply only
when rendering. *(Render output isn't covered here.)*

**SDV-04 — Levels mean different things for the two kinds (corrected in
v2).** Both start by splitting each n-sided polygon into n quads (a new
point on each edge and one in the middle).

- Catmull-Clark level L repeats that split L times: a quad becomes 4^L
  quads, a triangle 3·4^(L−1), a pentagon 5·4^(L−1). Unit cube: level 1 →
  26 points, level 2 → 98, level 3 → 386.
- Subdivision level L splits once, then divides each of those quads into an
  L × L grid: a quad becomes 4·L² quads, a triangle 3·L², a pentagon 5·L².
  Unit cube: level 1 → 26 points, level 2 → 98, level 3 → 218 (not 386).
  Levels 1 and 2 happen to give the same counts as Catmull-Clark.

## Catmull-Clark (Shift+Tab)

**SDV-05 — Catmull-Clark polygons follow the standard Catmull-Clark rules
exactly.** Each level computes: a face point = average of the polygon's
points; an edge point = average of the edge's two ends and the two adjacent
face points; a moved original point = (F + 2R + (n − 3)P) / n, where n is
the number of edges at the point, F the average of the adjacent face
points, R the average of the adjacent edge midpoints and P the original
position. This reproduced every frozen point, to six decimals, for a cube,
an asymmetric cube, a tetrahedron (all triangles), a pentagonal prism
(n-gon caps), a 6×6 grid with one raised point and an open 2×2 grid, at
levels 1 to 3. (SDV-O-PSUB-*.)

**SDV-06 — Boundary Rule decides open-edge behaviour.** On open boundaries
an edge point is the edge midpoint, and a boundary point that has two
boundary edges moves to (P_prev + 6P + P_next)/8 (the curve rule).

| Boundary Rule | Corner points (a boundary point used by only one polygon) |
|---|---|
| Crease (default) | Stay where they are |
| Smooth | Follow the same curve rule as other boundary points |
| Crease Edges | Same as Smooth on the tested grid without edge weights *(open: SDV-Q03)* |

(SDV-O-PSUB-grid-L1-crease / -smooth / -creaseEdges.)

**SDV-07 — Edge weight makes an edge semi-sharp; sharpness = 10 × weight.**
An edge weight w (the Subdivision weight map's edge value) gives the edge a
sharpness s = 10w, applied with the usual semi-sharp crease rules:

- an edge with s ≥ 1 gets its edge point at the plain midpoint; with
  0 < s < 1 the edge point is the smooth edge point moved toward the
  midpoint by the fraction s;
- the two child edges carry sharpness s − 1 to the next level (never below
  0);
- a point with exactly two sharp edges uses the crease rule
  (A + 6P + B)/8 along them, blended with the smooth rule by the average
  of the two sharpnesses when that is below 1; a point with one sharp edge
  uses the smooth rule; a point with three or more stays put (the standard
  rule; this case was not in the probe).

Fitted and confirmed on a cube with one weighted edge for w = 0.05, 0.1,
0.15, 0.25, 0.5, 1.0 at levels 1–3 (only 10 fits all seven cases). A
negative weight (−0.5) had no effect. (SDV-O-PSUB-cube-L*-edgew*.)

## Modo Subdivision (Tab)

**SDV-08 — Modo's own Subdivision gives a different, smoother surface.** On
a unit cube at level 1 the corner moves to 0.236111 (Catmull-Clark:
0.277778) and the face centre to 0.381944 (Catmull-Clark: 0.5).

**SDV-09 — The Subdivision surface: one Catmull-Clark step, then a
biquadratic patch on each quad (v2).** To build the surface of the smooth
polygons:

1. Apply one Catmull-Clark step (SDV-05) to the cage. Open boundaries use the
   curve rule for **every** boundary point, corners included (the Smooth
   rule; the mesh's Boundary Rule setting does not apply to this kind).
   The result is all quads.
2. For each quad q of that result, build a biquadratic Bézier patch with 3 × 3
   control points:
   - centre control = the centroid of q (average of its four points);
   - the control on each edge of q = the average of the centroids of the two
     quads sharing that edge; on an open boundary edge, the edge's midpoint;
   - the control at each corner of q = the average of the centroids of all
     quads around that point; at a point on an open boundary, the average of
     the controls of its two boundary edges (that is, of their midpoints).
3. At level L, sample each patch at parameters (i/L, j/L), i, j = 0…L, and
   connect the samples into L × L quads. Shared edges and corners give the
   same samples from both sides, so the result is watertight.

This reproduces every frozen point, to six decimals, for the cube,
asymmetric cube, tetrahedron, pentagonal prism, pentagonal bipyramid
(valence-5 points), the 6 × 6 impulse grid (with its open border) and the
open 2 × 2 grid, at levels 1, 2 and 3 (SDV-O-SUBD-*). In regular regions it
equals quintic B-spline refinement at level 1 (old points (3, 10, 3)/16, new
edge points (1, 15, 15, 1)/32 in each direction), which is why level 1
looked like "one Catmull-Clark step plus an averaging pass": step 2's corner
controls are exactly that average.

**SDV-10 — Higher levels sample the same patches more finely.** Because the
patches don't depend on the level, every level-1 point stays put at every
higher level whose grid contains its parameter (all levels for corners,
even levels for patch midpoints). (Replaces v1's open statement.)

**SDV-11 — Edge weights also work on Subdivision polygons, differently (still open).** A
weight on one cube edge moves nine points at level 1 (the edge's own points
and their neighbours), pulling the surface toward the edge progressively
with the weight. Unlike Catmull-Clark, a negative weight (−0.5) also changes
the result (11 points move), so negative values are meaningful here. Moving
only the Catmull-Clark edge point of step 1 does not explain the results;
the weight must also enter step 2. *(open: SDV-Q02)* Oracles
SDV-O-SUBD-cube-L*-edgew* pin the results. *(open: SDV-Q02)*

## Freezing

**SDV-12 — Freeze replaces smooth polygons with the display-level result.**
Every smooth polygon becomes plain Face quads at the display level (not the
render level), using SDV-04's topology; the result is what the oracles list.
What Freeze does with UV maps and edge weights is not covered yet *(open:
SDV-Q06)*.

## Mixed meshes and Freeze details (v3)

**SDV-13 — Face polygons cut the smooth region off (Q07).** In a mesh that
mixes Face and smooth polygons, the smooth polygons are subdivided as if the
Face polygons weren't there: their shared edges become open boundaries of
the smooth region (with that kind's boundary handling, SDV-06 / SDV-09),
and the Face polygons keep their original points. The two parts no longer
share points along those edges, so a gap can open. Unit cube with the +Z
face Face and the other five Catmull-Clark (or Subdivision), level 1: 29
points and 21 polygons; the 25 smooth-region points equal an open five-face
box subdivided on its own, plus the Face polygon's 4 unchanged corners.
(SDV-O-MIX-psub5-face1-L1, SDV-O-MIX-subd5-face1-L1.)

**SDV-14 — A mesh mixing Subdivision and Catmull-Clark polygons stays
connected, but its rule isn't established.** Cube with alternating kinds at
level 1: 26 points, 24 quads, matching neither all-Catmull-Clark nor
all-Subdivision. The oracle pins it. *(open: SDV-Q07)*

**SDV-15 — Freeze keeps UV maps (Q06, partial).** After freezing a Cube-tool
cube (one UV island per face) at level 1, the Texture map is still there for
every new polygon, and the UVs equal a straight linear subdivision of each
face's UV square. Islands of one quad can't show smoothing, so how UVs are
smoothed across larger islands, and the effect of Linear UVs and UV Boundary
Rule, remain open. Edge weights after Freeze weren't checked. *(open:
SDV-Q06)*
