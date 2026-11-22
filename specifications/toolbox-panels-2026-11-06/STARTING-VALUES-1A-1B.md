# Starting values for batches 1A and 1B (2026-11-06)

Read from the reference application's tool definitions through its
scripting interface (read-only: no tool was switched on or applied).
These are the values a freshly made tool starts with. REA-assisted: no.
Where a value is a choice, the first listed choice in the spec is meant
unless stated.

## 1A Slicing

**Polygon Slice:** Start (−1, 0, 0), End (1, 0, 0); Angle Snap **on**,
Angle **45°**; Fast Slice off; Split off; Cap Sections off; Gap 0; Offset
Side = first choice (centred: PS-Q2, label not confirmed); Infinite
**off**. Plane orientation default: perpendicular to the work plane, Y up.

**Edge Slice:** Split at Middle off; Split Polygons **on**; Show
**Position**; Offset 0; Snap Value **0.5%**; Inside Polygons **on**;
Always Raycast **on**; Multi Slice off.

**Loop Slice:** Edit **Move**; Mode **Free**; Current 0 (first slice);
Count **1**; Position **50%**; slider bar Length **200 px** at Slider X
**20 px**, Y **50 px**; Slice Selected off; Keep Quads off; Slice N-gon
off; Inset 0; Reverse Direction off; Reverse Inset off; Keep Aspect
**on**; Preserve Curvature off; Tension **100%**; Split off; Cap Sections
off; Gap 0; Select New Polygon **on**; sliced edges drawn highlighted
(on).

## 1B Deform

**Smooth:** Strength **1.0**; Iterations **1**; Thread off; Slip UVs off;
Preserve Volume off; Lock Boundary off; Lock Corner **on**; Lock Sharp
Edges off; Sharp Angle **60°**.

**Push:** Distance 0; Selection Normals **Averaged**.

**Rotate part (Twist, Swirl):** Angle 0°; Slip UVs off; Morph = first
choice.

**Linear falloff (factory, before any preset or auto-size):** Start
(0, 0, 0), End (0, 1, 0); Shape **Linear**; In 0, Out 0; Mix first
choice; Symmetric **None**. The Shear, Twist, Taper and Flare presets turn
Auto Size on, so the first click replaces Start/End anyway.

**Radial falloff (factory):** Centre (0, 0, 0); Size (1, 1, 1); Shape
**Linear**. Bulge's preset uses Shape **Smooth** instead. Soft Move,
Swirl use Linear.

**Transform part (Soft Move, Shear, Taper, Bulge):** Position 0, Scale
100%. Soft Move and Shear drag = Translate; Taper and Bulge drag = Scale.
