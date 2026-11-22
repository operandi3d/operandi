# Profiles and modes (edge 6–7)

**EBC-P-01 Round profile.** The extra points lie on a circular arc of
radius = Inset, centered on the line where the two inset lines meet
((y, z) = (0.4, 0.4) for edge 6–7), between the strip's two sides.
- Edge Bevel Round Level 2 gives 4 segments: points at (y, z) =
  (0.438268, 0.492388), (0.470711, 0.470711), (0.492388, 0.438268), i.e.
  every 22.5°. 16 points, 10 polygons; the end faces become octagons.
- Edge Chamfer Segments 3 gives 3 segments, points every 30°:
  (0.45, 0.486603), (0.486603, 0.45). 14 points, 9 polygons.
> v2 draft: the 2^L inference in the following sentence is superseded by EBC-B3-02. Original Level 0/2 measurements remain valid.

So Round Level L gives 2^L segments **(inferred from levels 0 and 2)**, and
Segments gives that many segments directly.

**EBC-P-02 Square shape keeps the original edge as the profile's corner.**
Edge Bevel Level 2, Square: 12 points, 8 polygons; the profile goes from
(0.4, 0.5) to the original corner (0.5, 0.5) to (0.5, 0.4) in (y, z) order
on each end. Edge Chamfer Segments 3, Square: 12 points, 10 polygons: two
strip quads through the original edge and a triangle at each end. Edge
Chamfer selects the original edge 6–7 afterwards in this case.

**EBC-P-03 Depth flattens the round profile toward the straight chamfer.**
Edge Chamfer Segments 3, Depth 0.5: points (0.440849, 0.477452) and
(0.477452, 0.440849) instead of (0.45, 0.486603) and (0.486603, 0.45).

**EBC-P-04 Inset vs Width.** With value 0.1:
- Inset mode (both tools): each side moves 0.1 along its face.
- Edge Bevel Width: each side moves 0.141421 along its face (strip width
  0.2 across the 90° corner).
- Edge Chamfer Width: each side moves 0.070711 (strip width 0.1).
*(So the two tools read Width differently; confirm at another angle:
EBC-Q02.)*

**EBC-P-05** Edge Chamfer with Stop At Edge off or Auto Weld on gave the
same result as EBC-J-01 for a single edge.
