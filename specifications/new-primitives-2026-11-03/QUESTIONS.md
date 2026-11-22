# New primitives — open questions (v1 draft)

- NPR-Q01 — Viewport: click-and-drag creation and handles for all four
  tools (Arc's two handles, Tube's Add / Edit / Delete point modes, Torus
  and Capsule bounding handles, Handle Mode, Show Corner). Needs pointer input.
- NPR-Q02 — Polygon Type Subdivs and Catmull-Clark (Torus, Capsule); Flip;
  Make UVs layouts beyond the two reset cases; Distortion Compensation; Gaps.
- NPR-Q03 — Torus Bulge Top / Bulge Side formula (data in TOR-05-*).
- NPR-Q04 — Capsule Quad Grid ends for Sides other than 8 and End
  Segments other than 2, including odd Sides.
- NPR-Q05 — Capsule Bulge formula (data in CAP-04-bulge0.5).
- NPR-Q06 — How Arc Center, Start, End, Radius and Angle update each other
  when one is changed (ARC-05-center shows Start/End shifting oddly).
- NPR-Q07 — Answered in NPR-ARC-04: Reverse gives the complement (360° − Angle).
- NPR-Q08 — Tube curve type through the control points, the By Length
  spacing and Twist rule (exact data in TUB-02-three-seg4, TUB-05, TUB-06);
  ring orientation along a bent path; profiles (Scale, Repeat, Reverse).
- NPR-Q09 — Capsule Axis X with the reset Radius Y 0.75 made 26 points
  instead of 42 (CAP-04-axisX): how Axis interacts with the size and End
  Segments values. Axis cases for Tube and Arc not observed.
- NPR-Q10 — Text: deferred. Its output depends on the fonts installed on
  the machine, and it makes a special text polygon (one 3-point polygon for
  "A" with the default font) rather than ordinary outlines. Needs Jason's
  decision on scope.
- NPR-Q11 — Primitives created into a non-empty or transformed mesh; undo.
