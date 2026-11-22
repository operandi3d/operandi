# UV rules seen across the tools

**UVN-R-01 Interpolation on the edited surface.** A new point that lies on a
surviving polygon (inset along an edge, cut into an edge, placed between two
borders) gets, in that polygon, the UV that the polygon's map gives at the
point's position. On G3 with continuous UVs every such corner equals
(x/3, y/3) exactly, for Edge Bevel, Edge Chamfer, Edge Extrude, Vertex Bevel,
Vertex Extrude and Bridge. *(The fixture's UVs are an affine function of
position, so linear interpolation along the edge and evaluation at the
position give the same numbers; which one Modo uses on curved UV layouts is
open: UVN-Q01.)*

**UVN-R-02 Seams survive in existing polygons.** With a seam, each existing
polygon's new corners use that polygon's own UVs: left polygons get
(x/3, y/3), seam polygons (x/3 + 1, y/3). No existing polygon on either
side changed frame, except in Fill Quads (UVN-T-07).

**UVN-R-03 Raised points copy their source.** Points lifted off the surface
by Edge Extrude's or Vertex Extrude's Shift get the UV of the original point
they rise from, in the frame of each wall polygon. Wall polygons are
therefore flat (zero width) in UV across the shift direction.

**UVN-R-04 Grown points continue the map.** Grow Quads' new row is placed
one row step beyond the border (y = 4 on G3) and its points get the map
value continued to that position: (x/3, 4/3).

**UVN-R-05 Points moved by Chamfer Edit keep their UVs.** Chamfer Edit moves
the strip's border points outward (x 1 → 0.9 and x 2 → 2.1 for Offset 0.1)
without changing their UVs ((1/3, …) and (2/3, …) stay), and the new
middle points get UVs between the borders' original values ((0.5, y/3) at
x = 1.5).

**UVN-R-06 New polygons that span a seam** get per-corner values from more
than one frame; each tool's choice is listed in BEHAVIOR-PER-TOOL.md and in
ORACLES.json. In general: corners inherited from an original point take that
point's continuous value; corners new to one side take that side's frame.
