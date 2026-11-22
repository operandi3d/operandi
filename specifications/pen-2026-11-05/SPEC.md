# Pen — numeric entry and options, behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

Scope: what the Pen tool builds from a given list of points under each
option. How points get into the list by mouse (clicking, Shift+click,
dragging, Ctrl) is covered by the Draw spec (PEN-01…PEN-07) and is not
repeated here.

## Points and numeric entry

The Pen keeps an ordered list of the points placed in the current session.
**Current Point** picks one point by its index (0-based, creation order);
**Position X/Y/Z** shows and edits that point. All cases here enter four
points with these fields and then drop the tool; the result is what the
same four clicks would build.

## Options and defaults (after Reset)

| Option | Default | Values |
|---|---|---|
| Type | Polygon | Polygon, Vertices, Spline Patch, Subdivision, Polyline, Catmull-Clark, B-Spline (this order) |
| Flip Polygon | off | |
| Close | off | |
| Merge | on | |
| Make Quads | off | |
| Wall Mode | Off | Off, Inner, Outer, Both |
| Offset | 0 m | wall half-width |
| Inset | 0 m | corner rounding radius |
| Segments | 10 | corner rounding segments |
| Make UVs | off | |
| Project To | Backdrop Item (inferred from the stored value) | Action Axis, Backdrop Item, U/V Direction |
| Show Angles / Handles / Numbers | off | display only |
| Select New | on | |
| Straight-line / axis / corner snap | on | interactive aids |

The guide also lists a "Lines" type (two-point segments); Modo 17.1 has no
such entry (Polyline is the nearest).

## Rules

1. **Polygon (default).** One polygon through the points in entry order;
   points are not reordered (PN01). Counter-clockwise entry seen from +Z
   gives a polygon facing +Z. The corner list may start at any point (Modo
   starts at point 1); compare up to rotation.
2. **Flip Polygon** reverses the corner order (PN02).
3. **Close** does nothing for Polygon type (a polygon is already closed,
   PN03) and nothing for Vertices (PN05).
4. **Types.** Vertices: points only, no polygon (PN04-1). Spline Patch,
   Subdivision, Catmull-Clark: one polygon of that type, same order as
   Polygon (PN04-2/3/5). Polyline and B-Spline: one open curve through the
   points in entry order, corner list starting at point 0 (PN04-4/6). An
   out-of-range type makes only points (PN04-7).
5. **Make Quads** with four points gives the same single quad as Polygon
   (PN06). Strip behaviour with more points: Draw spec PEN-06.
6. **Merge** does not merge points entered numerically on top of each
   other: a list whose last point equals the first keeps both (PN14 = PN15).
   Merge acts on dragging (Draw spec PEN-02).
7. **Make UVs** with Project To at its default and no backdrop: each
   point's UV equals its world X and Y (PN12); wall polygons likewise
   (PN13).

## Wall Mode

The points become the centre line of a strip of quads, one quad per
segment, all facing the same way as a Polygon drawn with the same points
(+Z for counter-clockwise entry). Left/right below are relative to the
direction of travel.

8. **Sides.** Inner puts the strip on the left of the line at distance
   Offset (the line itself is the other edge) (PN07-1). Outer puts it on
   the right (PN07-2). Both offsets both edges by Offset, so the strip is
   2 × Offset wide, centred on the line (PN07-3).
9. **Corners are mitred:** each offset edge is the segment moved sideways
   by Offset; at an interior point the two moved edges meet at their
   intersection (e.g. (0.9, 0.1) and (1.1, −0.1) for the corner (1, 0) at
   Offset 0.1). Open ends are offset at right angles to the end segment.
10. **Close on** adds the segment from the last point back to the first,
    mitred at the first point too (4 quads for 4 points, PN08).
11. **Point order:** per line point, the left point then the right point;
    each quad is `[left_i, right_i, right_i+1, left_i+1]` (PN07-3).
12. **Flip Polygon** reverses each wall quad (PN11).

## Wall corner rounding (Inset, Segments)

13. With Inset > 0 each mitred corner on the outside of the bend is
    replaced by an arc of radius Inset tangent to both edges, built from
    Segments straight pieces (Segments + 1 points). Segments 1 is a flat
    chamfer: corner (1.1, −0.1) with Inset 0.1 becomes (1.0, −0.1) and
    (1.1, 0.0) (PN09). Segments 3: arc points at 0°, 30°, 60°, 90° about
    the arc centre (PN10).
14. **Both:** the inside edge of the bend is rounded the same way (arc of
    radius Inset tangent to the inner edges), so both sides have the same
    number of points and the corner is a run of quads (PN09, PN10).
15. **Inner:** only the side on the outside of the bend (here the drawn
    line) is rounded; the inner offset corner stays one mitred point, and
    the corner is filled with a fan of triangles from it (PN11).

## Not covered

Interactive placement, Make Quads strips, Close on curve types, Project
To Action Axis / U/V Direction, Show options, Select New, undo. See
QUESTIONS.md.

## Open questions and decisions

- **Q-PEN-01** Outer wall mode with Inset: is only the outside of the bend
  rounded (as with Inner), and which side is "outside" for a right-hand
  bend? Needs a case with bends in both directions.
- **Q-PEN-02** Close on Polyline and B-Spline types: closed curve or
  ignored?
- **Q-PEN-03** Project To Action Axis and U/V Direction (wall UVs along the
  strip): values not measured.
- **Q-PEN-04** Make Quads with 5+ points numerically (the strip rule is
  only known from the hands-on PEN-06).
- **Q-PEN-05** Polygon corner list starts at point 1 in Modo. Operandi may
  start at point 0; cases compare up to rotation.
- **Q-PEN-06** (decided below) The point list survives Reset and switching the tool off
  and on within a session. Should Operandi keep it (Modo-like) or start
  empty each time the tool is picked (sane)?

## Decided (2026-11-05)

- **Q-PEN-06 → decided:** Pen starts with an empty point list after Reset
  (and each time the tool is picked); Modo's surviving list is not copied.

Polygon type names in the cases: face = Face, subdiv = Subdivision, psubdiv = Catmull-Clark, spatch = Spline Patch, curve = Curve, line = Polyline/line, bspline = B-Spline, onePoint = one-point polygon.
