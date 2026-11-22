# Tool audit Tier 1: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report, on a throwaway scene. Each line: do this → expect that.

## Add Loop / Add Point (branch add-loop-slide)

1. Edge tab > Add Loop. Tool Properties shows Position, Split at the
   Middle, Both Sides, Absolute Distance, Offset from End, Distance 1/2,
   Preserve Curvature, Tension, Show Position, Show Distance, and greyed
   Keep Quads / Split / Cap Sections / Gap with a tooltip.
2. Cube (Segments 3). Tick Split at the Middle; click an edge near its end
   → loop at 50 %; dragging doesn't move it.
3. Untick it, tick Both Sides; click at about 25 % → two loops (25 % and
   75 %). Drag → both move, mirrored. Cmd+Z once removes both.
4. Both Sides, click at the middle → one loop.
5. Absolute Distance on, Distance 1 = 0.1 m: click anywhere on an edge →
   loop 0.1 m from one end; Offset from End flips the end. Drag → Distance
   1 field follows the pointer.
6. Show Distance on, Show Position off → readout shows metres only.
7. Cylinder (12 sides, 2 segments), Preserve Curvature on: click a side
   edge that runs around the cylinder → the new up-and-down loop's points
   sit on the circle, not on the flat edge. Tension 0 % → the flat cut.
8. Add Point: same options except Both Sides; Distance 2 greyed.
9. `edge.addLoop` in the command line with edges selected uses the panel's
   Position and Both Sides.

## Slide (branch add-loop-slide)

1. Select an edge loop, Slide. Panel shows Stop Negative Slide, greyed Loop
   Slide (on) and Merge Vertices (on), greyed Duplicate, Preserve
   Curvature, Tension, Slip UVs.
2. Stop Negative Slide on: drag backwards → nothing moves; drag past the
   next loop → stops and merges there.
3. Textured cube (checker): Slip UVs on, slide a loop → the checker stays
   still; off → the checker stretches.
4. Vertex Slide on one point: Loop Slide off, Percent 150 → the point
   passes its neighbour; on → Percent 150 is refused.
5. Each field edit or drag is one undo step.
