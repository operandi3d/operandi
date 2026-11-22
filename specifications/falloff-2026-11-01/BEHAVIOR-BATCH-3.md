# Falloff on seven additional tools — v3 draft

Modo 17.1v1, 2026-11-02. Scripted numeric applications with complete mesh
and selection readback; REA-assisted: no. Each weighted result has an
otherwise identical no-falloff control. No symmetry or snapping. Regular
5×1 XY quad strip, X=0…5, Y=0…1, point index=row×6+column. Unless stated
otherwise, Linear Start (1,0,0), End (3,0,0), Shape Linear, Mix Multiply:
weights at original X=1/2/3 are 1/0.5/0, clipped to 1 and 0 outside.

| Requirement | Tool | Measured response in this numeric case |
|---|---|---|
| FAL-B3-01 | Edge Extrude | Responds: Inset and Shift scale by each original endpoint's weight; zero weight also changes topology. |
| FAL-B3-02 | Vertex Extrude | Responds: Inset and Shift scale by the selected original point's weight; zero-weight point stays unextruded. |
| FAL-B3-03 | Spike | Responds: new tip height scales by the mean of the original face's corner weights; all-zero faces stay quads. |
| FAL-B3-04 | Edge Slide | Responds: 25% Radial slide moves a single interior edge by −0.25, −0.125 or 0 at weights 1,0.5,0. |
| FAL-B3-05 | Loop Slice | No response observed: the active one-cut Uniform case has identical geometry, topology and selection with/without falloff. |
| FAL-B3-06 | Bridge | Responds, with changed topology and collapsed intermediate coordinates; this case does not fit simple distance-times-weight interpolation. |
| FAL-B3-07 | Push | Responds: Distance 1 moves each original point in +Z by its weight, retaining topology. |

These are scoped numeric observations, not claims about every mode or
pointer operation. Every point, polygon and selected component is in the
corresponding oracle pair.

**FAL-B3-01 details.** Select the four interior vertical edges X=1,2,3,4.
Edge Extrude Inset 0.2, Shift 1: control 28 points / 13 faces, weighted
26/9. The X=1 ridge has Z=1 and cuts at X=0.8/1.2. X=2 ridge has Z=0.5
and cuts X=1.9/2.1. X=3/4 do not acquire raised ridges. Coincident extra
points occur at X=3 in the weighted mesh and are retained in the oracle;
zero falloff cannot be modeled as full extrusion followed by simple
vertex interpolation alone. FAL-O-F3-edge-extrude-control/linear.
Boundary-chain auxiliary cases are also recorded and show scaled Inset,
but that boundary selection creates no raised ridge even in the control.

**FAL-B3-02 details.** Select bottom vertices 1,2,3. Inset 0.2, Shift 1:
control 21/11; weighted 18/9. X=1 makes tip (1,0,1) and cuts 0.2 away;
X=2 makes tip (2,0,0.5) and cuts 0.1 away; X=3 remains (3,0,0).
FAL-O-F3-vertex-extrude-control/linear. Separate corner/border controls
preserve their distinct end behavior without extending it to interiors.

**FAL-B3-03 details.** Select all five faces, Spike Strength 1. Control
adds five height-one tips: 17 points / 20 triangular faces. With Start X=1,
End X=3, only the first three faces split into triangles; tips at X=0.5,
1.5,2.5 have Z=1,0.75,0.25, yielding 15/14. The last two faces remain
quads. FAL-O-F3-spike-control/linear. An extra Start X=0.5, End X=4.5
control makes all five tips with Z=0.9375,0.75,0.5,0.25,0.0625. The end
heights differ from evaluating the falloff at the face centers (which
would give 1 and 0); they match averaging the already-clipped corner
weights. FAL-O-F3-spike-centers-control/linear.

**FAL-B3-04 details.** Single interior vertical edge at X=1,2,3, tested
separately, Mode Radial, Percent 25. Controls move both endpoints left
0.25. Weighted endpoint X becomes 0.75,1.875,3 respectively. All cases
stay 12 points / 5 faces. FAL-O-F3-edge-slide-x1-control/linear,
x2-control/linear, x3-control/linear. Selecting all four parallel interior
edges only moved the first in the measured Radial case, so its identical
weight-one output is not used to infer falloff independence. A Linear-mode
no-motion driving control is also excluded from that inference.

**FAL-B3-05 details.** Select all bottom horizontal edges. Uniform, Count 1
makes one midline cut through each quad (new X=0.5,1.5,2.5,3.5,4.5,
Y=0 and 1), 22 points / 10 faces. In this mode the requested Position 25%
read back as 50%; both control and weighted cases use that observed
midpoint. Their complete outputs are equal, including zero-weight regions.
FAL-O-F3-loop-slice-control/linear. Other modes remain unmeasured.

**FAL-B3-06 details.** Bridge requires an opening: retain only the strip's
end quads, leaving original unused points in place. Select opposing edges
1–7 and 4–10. Curve, Segments 4, Tension 100%. Control 18/6: new point
pairs at X=1.75,2.5,3.25, Y=0/1. Weighted 14/5: only one new pair, both
at X=4, with faces [1,12,13,7], [12,13,12], [12,4,10,13] in addition to
the two original faces. A repeated-index degenerate triangle is part of
this measured result. FAL-O-F3-bridge-control/linear. A setup rejected
Inset without a profile was stopped before application and is not evidence
of Bridge behavior. Broader Bridge/falloff rules remain open.

**FAL-B3-07 details.** Select all points, Distance 1, Selection Normals
Averaged. Control Z=1 everywhere. Weighted Z=1,1,0.5,0,0,0 along each
row; X/Y and all faces unchanged, 12/5. FAL-O-F3-push-control/linear.
