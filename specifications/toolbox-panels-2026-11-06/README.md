# Toolbox panels: Deform, Duplicate, Edit (2026-11-06)

Behaviour-only specs for three toolbox tabs. **v0: docs + plain-text UI
config only; nothing observed by script or by hand yet.** REA-assisted: no.

- `DEFORM.md`: Deform tab (25 buttons incl. flyouts).
- `DUPLICATE.md`: Duplicate tab (43 buttons incl. flyouts).
- `EDIT.md`: Edit tab (30 buttons).

Each file has the panel layout in words (sections, order, labels,
shortcuts, flyouts, modifier-click variants), then per tool: what it does,
how it's driven, its options (with starting values where known), status in
Operandi, and open questions.

Status tags: **Exists**, **Partial**, **New**, **Check** (couldn't confirm
from the research side; the build side should look).

## Two shortcuts that make this cheaper than it looks

1. **Most Deform buttons are presets**, not tools: an existing transform
   plus one falloff. Operandi has Move/Rotate/Scale and Linear + Radial
   falloffs, so Soft Move, Shear, Taper, Twist, Swirl and Bulge are wiring.
2. **Most Duplicate buttons share one engine**: a placer (line, grid,
   circle, curve, random cloud, mirror plane) plus a filler (copy
   geometry, sweep, copy items, instance, transform existing items).
   Build it once, then each button is a pair.

## Build tiers (simplest and most used first)

### Tier 1: everyday, mostly built from parts Operandi has

| Batch | Tools | Why first |
|---|---|---|
| **1A Slicing** | Loop Slice, Edge Slice, Polygon Slice | Used constantly; the Edit tab has nothing yet. Loop Slice is probably used more than Add Loop. |
| **1B Deform presets** | Soft Move, Shear, Taper, Twist, Swirl, Bulge, plus Push (+Flare) and Smooth | One pattern for six buttons; Push and Smooth are small new tools. |
| **1C Duplicate core** | Mirror, Clone, Array, Radial Array (geometry versions) | Core for symmetrical and repeated modelling; builds the placer/filler engine. |
| **1D Small wiring** | Center X/Y/Z/All, one adaptive Slide button, Subdivide buttons, Element Push | Commands and tools that mostly exist already. |

### Tier 2: regular use, some new machinery

- **Deform:** Bend, Flex (needs Selection falloff), Soft Drag (Screen
  falloff), Vortex (Cylinder falloff), Jitter, Linear Align, Radial Align,
  Relax Mesh.
- **Duplicate:** Radial Sweep, Curve Extrude (+Bezier, B-Spline), Curve
  Clone + Curve Transform, Scatter Clone, Tack Tool, Symmetrize, Bridge
  options (already partial; see tool audit).
- **Edit:** Booleans (Boolean, Solid Drill, Axis Drill, Curve Boolean),
  Polygon Axis Slice, Pen Slice, Curve Slice, Primitive Slice,
  UnSubdivide, Freeze dialog.

### Tier 3: niche or big

- **Deform:** Soft Selection Move/Rotate/Scale/Transform, Quantize,
  Sculpt (belongs with the planned Sculpt work).
- **Duplicate:** every Instance / Replica / Transform variant (needs
  instances and replicators), Mesh Paint.
- **Edit:** Julienne, Dicer (both need solid booleans), Fix Black Dots,
  Edge Zip, Diagonal Re-Direct, Automatic Retopology + Retopo Edge Lock
  / Flow, Conditional Commands.

## Already in Operandi

Element Move (Tweak, partial), Quick Align, Ground Align, Paste (Paste
Tool), Bridge (partial), Slide (as two tools, partial), Mesh Cleanup, New
Mesh, Subdivide and subdivision Freeze (check the buttons and options).

## Before building

A single read-only scripted pass in the reference (no applies, no crashes
risked) would fill the starting values the docs don't give (DF-Q1, DP-Q3,
ED-Q1). The geometric open questions (twist blending, bend arc, Loop Slice
first activation, Center rule) need a few small scripted cases each.
