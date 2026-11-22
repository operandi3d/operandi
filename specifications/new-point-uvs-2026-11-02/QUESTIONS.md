# New-point UVs: open questions

Answered here:

- **SQ-UV-01** for UVs: answered per tool (BEHAVIOR-UV-RULES,
  BEHAVIOR-PER-TOOL, 18 oracles). Compared with Operandi's provisional rule:
  interpolation inside each polygon, seams kept, raised points reusing the
  edge ends' UVs, Grow Quads continuing by one row step, Bridge blending
  between borders and Chamfer Edit keeping corners all match. Differences:
  Modo's Edge Bevel strip takes one neighbour's frame on all corners
  (not a blend); Chamfer Edit moves border points without updating their UVs;
  Fill Quads can change an unselected neighbour's UV at a seam; Edge Chamfer
  leaves coincident zero-area end triangles.

Current question status and original recipes:

- **UVN-Q01 — Partly answered (UVN-B2-01):** non-affine source-edge interpolation measured; general curved polygon interiors remain open. Original recipe: interpolation vs evaluation on a non-affine UV layout. Re-run
  Edge Bevel on G3 with point 5's UV moved to (0.5, 0.5).
- UVN-Q02: corner weights, vertex colors, normals and typed corner maps
  (**EX-Q03, BV-Q04** for the other tools). The research side has not yet
  found a reliable way to set up a weight map as test input, so these are
  unobserved.
- **UVN-Q03 — Answered for the requested left-seam fixture (UVN-B2-02):** right neighbor frame retained; other orientations remain open. Original recipe: which side's frame Edge Bevel's strip takes when the seam is on the
  other side (repeat with the left column as the seam polygons).
- **UVN-Q04 — Answered for Profile/Triangle Fan × Stop At Edge (UVN-B2-03):** only Profile has coincident zero-area ends in this fixture. Original recipe: Edge Chamfer's coincident end triangles: do they also appear with
  Stop At Edge or other end modes?
- UVN-Q05: undo/redo (not drivable by script).
