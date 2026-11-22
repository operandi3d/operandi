# Edge weights (subdivision creases): build spec v1, 2026-11-05

Behaviour only. IDs EW-xx are this hand-off's own. Where Operandi chooses its
own behaviour on purpose (Jason's bar: "close enough and sane"), the item says
**Operandi choice**.

## Data

**EW-01 — Each edge can carry one weight, stored per mesh.** A weight is a
number shown as a percentage (0 % = no weight, the default for every edge).
It lives in the mesh's "Subdivision" weight map, which every new mesh has,
as a value on the edge. Edges without a value count as 0 %. Negative values
are allowed and stored; they have no effect on Catmull-Clark (EW-03).
Suggested range for the UI: −100 % to 100 % (typed values outside can be
clamped; **Operandi choice**).

**EW-02 — Weights only affect smooth polygons.** On Face polygons a weight is
stored but changes nothing. A primitive's Edge Weight option is refused when
the primitive is made of Face polygons (message along the lines of "only
applies to subdivision surfaces"); the primitive is still made.

## Catmull-Clark rule (measured, exact)

**EW-03 — Sharpness s = 10 × weight (weight as a fraction: 20 % → s = 2).**
Semi-sharp crease rules, per subdivision step:

- Edge point: if s ≥ 1, the plain midpoint of the edge. If 0 < s < 1, the
  normal smooth edge point moved toward the midpoint by the fraction s.
- Both child edges carry s − 1 into the next step (never below 0).
- Original point with exactly two sharp edges (s > 0): crease rule
  (A + 6P + B) / 8, where A and B are the far ends of the two sharp edges.
  If the average of those two sharpnesses is below 1, blend: that average ×
  crease result + (1 − average) × smooth result.
- Original point with one sharp edge: smooth rule.
- Original point with three or more sharp edges: stays where it is.
- Weight ≤ 0: no effect.

Plain-language check: a 20 % edge stays fully hard for 2 levels, a 30 % edge
for 3, and then rounds off. Tutorial footage says exactly this.

Tests: ORACLES-EDGE-WEIGHTS.json, `targets` (unit cube with one weighted
edge at 5, 10, 15, 25, 50, 100 % and −50 %, levels 1–3; six decimals).

**EW-04 — Boundary Rule "Crease Edges".** On meshes with this rule, treat
open boundary edges as fully sharp (s = infinity) and corner points as
fixed. On the tested grid with no weights it gives the same result as
Smooth (oracle SDV-O-PSUB-grid-L1-creaseEdges); match that oracle.
**Operandi choice** for anything beyond it.

## Modo's own Subdivision kind (Tab polygons)

**EW-05 — Use the same rule for both kinds. Operandi choice.** Modo's own
result for weighted Subdivision polygons is not understood. Operandi uses:

1. Step 1 (the single Catmull-Clark step of SDV-09) applies EW-03 exactly.
2. In the patch stage, any quad edge that lies on a child edge whose carried
   sharpness is still ≥ 1 is treated like an open boundary edge: its edge
   control is the edge midpoint, and a corner on two such edges uses the
   average of those two edges' midpoints. Carried sharpness between 0 and 1
   blends between the boundary control and the normal control by that
   fraction. Zero: normal patch.
3. Higher levels sample the same patches (SDV-10), so no further carry-down.

Result: weighted edges look hard and round off as the weight drops, which is
what users expect. It will not match Modo point for point.

**EW-06 — Don't test against Modo's Subdivision-kind weight results.**
They are in `reference_only` for curiosity. Note: in Modo a negative weight
does change this kind (pulls the surface away); Operandi ignores negatives
for both kinds. **Operandi choice.**

## Tool and display

**EW-07 — Edge Weight tool.** In Edges mode, listed in the edge right-click
Modeling Operations menu after Add Loop (a separator follows it). Drag in
the viewport to raise or lower the weight of the selected edges; the panel
shows one number, Weight (%). With nothing selected, do whatever
Operandi's other edge tools do with an empty selection (**Operandi
choice**). One drag = one undo step. If Operandi's existing Edge Crease
tool already does this, rename or merge it rather than adding a second tool.

**EW-08 — Seeing weights.** Weighted edges get a highlight colour in the
viewport, brighter for higher weights (**Operandi choice** until vertex-map
display exists; a theme colour, not hard-coded).

**EW-09 — Tools that make new edges don't copy weights.** Bridge does not
carry weights onto the edges it makes (reported in tutorials). Do the same
for every tool for now: new edges are 0 %. Splitting an edge (Add Loop,
Slice) gives both halves the original weight (**Operandi choice**, so a
crease survives a cut).

**EW-10 — Freeze.** Freeze turns smooth polygons into Face polygons, so
weights stop mattering. Drop the weights on frozen edges. **Operandi
choice** (Modo's behaviour unmeasured).

## Files

LXO: Modo keeps subdivision weights in its Subdivision weight map (the
legacy-import spec calls these SUBV maps). Exactly how edge values are laid
out in the file is not researched here; if the LXO code can't already read
them, leave LXO weights for a later pass and keep listing them in the import
note, as the legacy-import spec says.
OBJ/STL/glTF: no weights (glTF could, later).

## Not covered

Vertex weights (points of the same map), weights as a generic vertex map,
weight display through a vertex-map shading mode.
