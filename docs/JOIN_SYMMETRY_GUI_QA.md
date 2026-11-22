# Join with Symmetry — GUI check

QA build: `~/Developer/Operandi-Builds/Operandi join-symmetry <commit>.app`.

- **JS-01** Symmetry X on. Make a mesh with a gap between two rows of
  polygons on each side (or use the cartoon kid scene). In Edge mode click
  one edge, then the edge across the gap: four edges highlight. run Join from the Edge commands:
  both sides close, the mesh stays symmetric, one Cmd+Z undoes it.
- **JS-02** Same with Join Averaged: both sides meet halfway.
- **JS-03** Pick two edges that end on the centre line (and their mirrors):
  Join closes the gap across the whole width, with no doubled point at the
  centre.
- **JS-04** Pick a single pair lying across the centre line: Join works.
- **JS-05** Pick three edges: Join refuses with "Join needs two selected
  edges, or two on each side with Symmetry on (selected: …)".
- **JS-06** Vertex mode: pick a point and the point it should join to;
  four points highlight. Join: each side welds onto its own second pick.
