# New primitives — behavior (v1 draft)

All four tools add new geometry to the active mesh item. Nothing is
selected afterwards in any mode (every oracle). Angles below go around the
Y axis measured from +X toward +Z, so -90° is the -Z direction. "Around"
order means -Z first, then -X, +Z, +X (decreasing angle).

## Reset values

| Tool | Values after Reset |
|---|---|
| Torus | Position 0,0,0; Radius X/Y/Z 0.25; Sides 24; Segments 12; Ring Radius 0.5; Cross Section 0.25; Height 0.25; Hole Size 2; Bulge Top 0; Bulge Side 0; Axis Y; Polygon Type Face; Make UVs on; Flip off; Distortion Compensation off; Gaps 0 |
| Capsule | Position 0,0,0; Radius X/Y/Z 0.25 / 0.75 / 0.25; Sides 24; Segments 1; End Segments 5; End Size 0.25; Ends Polar; Bulge 0; Axis Y; Polygon Type Face; Make UVs on; Flip off; Distortion Compensation off; Gaps 0 |
| Arc | Center 0,0,0; Start (1,0,0); End (0,1,0); Radius 1; Angle 90°; Segments 15; Reverse off |
| Tube | Sides 12; Segments 10; Radius 0.2; Caps on; Closed off; By Length off; Twist 0°; Make UVs on; Mode Add; profile Scale 50 %, Repeat on, Reverse Inset off, Reverse Direction off; no control points |

The guide lists Capsule Segments as 8 and End Size as 50 cm; the reset
values observed are 1 and 0.25.

## Torus

**NPR-TOR-01 Shape.** A ring of radius R (Ring Radius) around the axis,
swept by an elliptical cross-section with radial half-width w (Cross
Section) and half-height h (Height). Points come in Segments rows of Sides
points. Row k (k = 0 … Segments−1) sits at cross-section angle
φ = 360°·k/Segments, starting on the inner equator and going next toward
the bottom: distance from the axis R − w·cos φ, height −h·sin φ. Within a
row, point i (i = 0 … Sides−1) is at angle −90° − 360°·i/Sides around the
axis. Position offsets everything. Exact to 1e-6 in all Axis-Y, no-bulge
cases (TOR-01, 02, 03-*, 06).

**NPR-TOR-02 Polygons.** One quad per (row k, step i), in row order then
step order: `[k·S+i, k·S+i+1, (k+1)·S+i+1, (k+1)·S+i]`, with i+1 wrapping
to 0 and row k+1 wrapping to row 0 (S = Sides). Every face points away
from the ring's core circle. 24 × 12 reset gives 288 points and 288 quads.

**NPR-TOR-03 Linked sizes.** Radius X and Z equal Cross Section, Radius Y
equals Height, and Hole Size always equals Ring Radius ÷ Cross Section.
Observed updates:
- Setting Hole Size keeps Cross Section and changes Ring Radius (hole 1 →
  ring 0.25, so the hole closes to the axis) (TOR-03-hole1).
- Setting Ring Radius keeps Cross Section and changes Hole Size (ring 1 →
  hole 4) (TOR-03-ring1).
- Setting Cross Section keeps Ring Radius and changes Hole Size; Radius X/Z
  follow (0.1 → hole 5) (TOR-03-xsec0.1).
- Setting Height changes only Radius Y (TOR-03-height0.5).
- Setting Radius X/Y/Z to 1 sets Cross Section and Height to 1 and keeps
  Hole Size 2, so Ring Radius becomes 2 (TOR-03-size1).

**NPR-TOR-04 Axis** X or Z turns the ring so its axis is X or Z
(TOR-04-axisX, -axisZ; exact point order in the oracles).

**NPR-TOR-05 Bulge.** Bulge Top 0.5 and Bulge Side 0.5 keep counts and
order but push the cross-section toward a box shape; Bulge Side 0.5
reaches distance 0.8038 from the axis instead of 0.75. Exact positions are
in TOR-05-*; the rule is open (NPR-Q03).

## Capsule

**NPR-CAP-01 Shape (Ends Polar).** Along the axis, with half-length L
(Radius Y), end height e (End Size) and radius r (Radius X/Z): a straight
body from −(L−e) to +(L−e) and a rounded end of height e at each side,
closed by a pole point at ±L. The body has Segments + 1 rings evenly
spaced. Each end has End Segments − 1 rings between body and pole: ring j
(j = 1 … End Segments − 1, counted from the pole) is at angle
a = 90°·j/End Segments, height (L−e) + e·cos a from the centre and radius
r·sin a. Ring points go around in the same order as the Torus (first at
−90°). Exact to 1e-6 in CAP-01, 02, 04-endsize, 04-segments1, 04-endseg1.

**NPR-CAP-02 Point and polygon order (Polar).** Bottom pole first, then
the rings from bottom to top, top pole last. Polygons: the bottom fan of
triangles `[ring point i, pole, ring point i+1]`, then quads band by band
upwards, then the top fan. Every face points outward. Counts: points
Sides·(Segments + 1 + 2·(End Segments − 1)) + 2, polygons
Sides·(Segments + 2·(End Segments − 1)) + 2·Sides triangles (CAP-02: 42 / 48).

**NPR-CAP-03 Ends Quad Grid** fills each end with quads only: for 8 sides
and 2 end segments, each end gets 4 quads around a centre point instead of
8 triangles (42 points, 40 quads; CAP-03-ends1). Point order differs from
Polar (the bottom centre point comes after the first ring). Layout for
other Sides counts is open (NPR-Q04).

**NPR-CAP-04** End Size 0.1 moves only the end rings (CAP-04-endsize0.1).
Bulge 0.5 keeps counts and swells the body (CAP-04-bulge0.5; rule open,
NPR-Q05). End Segments 1 gives no end rings: each end is a triangle fan
straight from the last body ring to the pole (CAP-04-endseg1).

## Arc

**NPR-ARC-01 Shape.** One flat polygon shaped like a pie wedge: the
corners run from Start along the circle of radius Radius around Center to
the end of the sweep, Segments steps of Angle/Segments each, then the
Center point closes it. Segments + 2 corners, in that order; the polygon
is `[0, 1, …, Segments+1]`. Reset: Start (1,0,0) sweeping toward +Y in the
XY plane, 90°, 15 segments, 17 corners (ARC-01).

**NPR-ARC-02 Plane and facing.** The wedge lies in the plane through
Center, Start and End, sweeping from Start toward End. Start (1,0,0) with
End (0,0,−1) gives a wedge in the XZ plane toward −Z (ARC-06). The face
points along the right-hand normal of the sweep (+Z for the reset arc).

**NPR-ARC-03 Angle is limited to 180°.** Entering 200°, 270° or 360°
gives 180° (ARC-07-angle200, ARC-02-r1-a270-s6, ARC-04-a360).

**NPR-ARC-04 Reverse** makes the rest of the disc instead: the sweep goes
the other way round from Start to the same End, covering 360° − Angle.
Angle 90° reversed gives four 67.5° steps through −Y back to (0,1,0)
(ARC-03-reverse); Angle 180° reversed gives the opposite half-disc
(ARC-07-angle200-reverse). The face points the opposite way.

**NPR-ARC-05** Setting Center after Reset moves the wedge but also
changes Start and End in a way not yet understood (ARC-05-center, NPR-Q06).

## Tube

**NPR-TUB-01 Shape.** A tube of radius Radius along a smooth curve
through the control points, in the order given. Each control point and
each in-between step carries a ring of Sides points; Segments is the
number of steps per span between consecutive control points. Caps on
closes each open end with one polygon of Sides corners. Closed joins the
last control point back to the first with one more span and has no caps.

**NPR-TUB-02 Counts.** Rings = spans × Segments + 1 (open) or
spans × Segments (closed); points = rings × Sides; polygons = (rings − 1)
× Sides quads (open) or rings × Sides (closed), plus 2 caps when on.
Two points straight up (0,0,0)→(0,1,0), 4 sides, 1 segment, radius 0.25:
rings at y 0 and 1, first ring (0.25,0,0), (0,0,−0.25), (−0.25,0,0),
(0,0,0.25) (TUB-01). Three points with 1 / 4 segments: 12 / 36 points
(TUB-02-*); caps off 20 points 16 quads (TUB-03); closed 24 / 24
(TUB-04).

**NPR-TUB-03** By Length spaces the rings evenly along the curve instead
of evenly per span (TUB-05-bylength; same counts, positions in the
oracle). Twist 90° turns the rings progressively along the tube
(TUB-06-twist90). The curve and twist rules are open (NPR-Q08).
