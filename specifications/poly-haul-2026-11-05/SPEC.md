# Poly Haul — behavior spec v0 (guide and settings only)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Poly Haul is one tool that switches between nine modelling operations
without dropping the tool. You drag on the element under the pointer;
Space opens its mini properties with a Mode list and that mode's main
controls. Numeric-keypad shortcuts pick the first six modes. When the
result intersects other geometry it is joined to it (a boolean union) by
default.

## Modes (guide; keypad key in brackets)

| Mode | What it does |
|---|---|
| Offset [0] (default) | Pushes or pulls the polygon along its normal without adding edge loops. Snap at Edge End snaps it to the end of connected edges. Falloff can weigh per point, per selection group or per polygon. |
| Extrude [1] | Like Offset but adds new side polygons. Holding Shift in Offset switches to Extrude temporarily. |
| Split [2] | Cuts the polygon in two. Position says where along the edge, Spin which edge. Stays inside one polygon. |
| Add Loop [3] | Like Split but continues around contiguous quads (and odd-sided surface polygons). |
| Scale [4] | Scales the polygon in its own X and Y, without extruding. |
| Bevel [5] | Extrude plus inset: Inset, Shift, Segments, Group Polygons. |
| Remove | Removes the vertex, edge or polygon; its points too, unless Keep Vertices is on. |
| Tear | Tears a polygon's corner off its neighbours (Select Through off). |
| Slide | Slides a point or edge along connected edges; Duplicate copies the point and triangulates the polygon around the copy. |

## Settings after Reset (PH-D0)

Mode **Offset**; Offset 0; Position 50 %; two distances 0.5 and 0.5; Snap
at Edge End **on**; Scale X/Y/Z 100 %; Inset 0; Shift 0; Segments 1; Group
Polygons **on**; Spin 0; Keep Vertices off; direction vector 0, 0, 0;
Duplicate off; push angle 1°; both sides off; boolean union **on**;
curvature off; middle off; quad off; absolute distance off; distance from
end off; Tension 100 %; split off; caps off; gap 0; Use Material off,
material name "Default"; thicken off; square off; edge rail off; even
offset **on**; maximum angle 89.9° (1.569051 rad); round level 0; shape
Round (Round, Square, Sharp); stop at edge **on**; auto weld off; depth
100 %; parallel **on**; coplanar off; edge end Profile (Profile,
Triangle Fan); flatness 0.00001; extend boundary **on**; mitering 0;
sharp corner off; falloff mode Vertex (Vertex, Group, Polygon). Many of
these have no guide description; they are listed so Operandi can name
them, not as behaviour.

## Rules

1. Only the mode list, the controls named above and the defaults are
   specified. All geometry results are open (Q-PH-01).

## Open questions and decisions

- **Q-PH-01 (decision pending)** Every mode's geometry. Scripted apply
  crashes Modo, so this needs a hands-on session: drag each mode on a cube
  face with fixed numeric values, then read the mesh back by script.
- **Q-PH-02** Which element is used when something is selected vs only
  hovered.
- **Q-PH-03** The boolean union on intersecting results.
- **Q-PH-04** Which settings each mode's mini properties show.
