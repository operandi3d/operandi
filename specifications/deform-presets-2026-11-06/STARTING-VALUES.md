# Starting values for batches 1A and 1B (2026-11-06)

Read from the reference application's tool definitions through its
scripting interface (read-only: no tool was switched on or applied).
These are the values a freshly made tool starts with. REA-assisted: no.
Where a value is a choice, the first listed choice in the spec is meant
unless stated.

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
