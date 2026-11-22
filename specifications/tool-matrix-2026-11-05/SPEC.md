# Model toolbox tool matrix — reference v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05. REA-assisted: no.

## What this is

Every button of Modo's Model toolbox (the vertical tool bar on the left of
the Model layout), grouped by tab and group, with the research spec that
describes its behaviour. It replaces the old modeling-toolset milestone
matrix as the clean planning reference. 243 buttons; 99 have a spec.

| Tab | Buttons | With a spec |
|---|---|---|
| Basic | 51 | 31 |
| Deform | 24 | 5 |
| Duplicate | 47 | 2 |
| Mesh Edit | 25 | 7 |
| Vertex | 18 | 18 |
| Edge | 19 | 11 |
| Polygon | 34 | 25 |
| Curve | 25 | 0 |

Notes:
- Some buttons appear on more than one tab (Bevel, Extrude, Bridge, Pen,
  Tack Tool); each place is listed.
- The Bevel button in Basic > Add Geometry switches to Polygon, Edge or
  Vertex Bevel according to the selection mode (see bevel-activation).
- Duplicate's Clone/Array/Scatter/Radial Array groups each have Instance,
  Replica and Transform variants; they're listed separately.
- "Covered by" names research specs; a spec may cover a button only partly
  (see each spec's status).

## Buttons

### Basic

| Group | Button | Covered by |
|---|---|---|
| PrimGang > Primitives | Cube Primitive | primitives-2026-11-02, unit-primitive-2026-11-05 |
| PrimGang > Primitives > Sphere-Ellipse | Sphere Primitive | primitives-2026-11-02, unit-primitive-2026-11-05 |
| PrimGang > Primitives > Sphere-Ellipse | Ellipsoid Primitive | ellipsoid-2026-11-05 |
| PrimGang > Primitives > Cyl-Caps | Cylinder Primitive | primitives-2026-11-02, unit-primitive-2026-11-05 |
| PrimGang > Primitives > Cyl-Caps | Capsule Primitive | primitives-2026-11-02, unit-primitive-2026-11-05 |
| PrimGang > Primitives > Cyl-Caps | Arc | create-panel-2026-11-04 / new-primitives-2026-11-03 |
| PrimGang > Primitives | Cone Primitive | cone-2026-11-05 |
| PrimGang > More Prim Tools | Torus Primitive | primitives-2026-11-02, unit-primitive-2026-11-05 |
| PrimGang > More Prim Tools > VolumeDrawing | Tube Primitive | primitives-2026-11-02, unit-primitive-2026-11-05 |
| PrimGang > More Prim Tools > VolumeDrawing | Solid Sketch | — |
| PrimGang > More Prim Tools > 2D Drawing | Pen | pen-2026-11-05, create-panel-2026-11-04 |
| PrimGang > More Prim Tools > 2D Drawing | Curve | — |
| PrimGang > More Prim Tools > 2D Drawing | B-Spline | — |
| PrimGang > More Prim Tools > 2D Drawing | Bezier Curve | — |
| PrimGang > More Prim Tools > 2D Drawing | Sketch | — |
| PrimGang > More Prim Tools > 2D Drawing | Patch Curves | — |
| PrimGang > More Prim Tools | Text | create-panel-2026-11-04 / new-primitives-2026-11-03 |
| Transform | Transform | item-transforms-2026-11-02, action-center-2026-11-01, viewport-handles-2026-11-01 |
| Transform | Move Tool | item-transforms-2026-11-02, action-center-2026-11-01, viewport-handles-2026-11-01 |
| Transform > Rotate | Rotate Tool | item-transforms-2026-11-02, action-center-2026-11-01, viewport-handles-2026-11-01 |
| Transform > Rotate | Axis Rotate | — |
| Transform > Scale | Scale Tool | item-transforms-2026-11-02, action-center-2026-11-01, viewport-handles-2026-11-01 |
| Transform > Scale | Uniform Scale | item-transforms-2026-11-02, action-center-2026-11-01, viewport-handles-2026-11-01 |
| More Transforms | Axis Rotate | — |
| More Transforms | Tack Tool | — |
| More Transforms | Element Snap | snapping-2026-11-01 |
| Snaps and Precision > Snap Modes | Grid | snapping-2026-11-01 |
| Snaps and Precision > Snap Modes | Geometry | snapping-2026-11-01 |
| Snaps and Precision > Snap Modes | Guides | snapping-2026-11-01 |
| Snaps and Precision | Drag Snap | snapping-2026-11-01 |
| Snaps and Precision | Drag Snap Rigid | snapping-2026-11-01 |
| Snaps and Precision | Planar Rotate Snap | — |
| Snaps and Precision | Planar Rotate Snap Axis | — |
| Snaps and Precision | Scale Absolute | — |
| Snaps and Precision | Size Absolute | — |
| Snaps and Precision | Advanced Handles | — |
| Snaps and Precision > Commands | Absolute Scaling | — |
| — | Vertex Normal Tools | — |
| — | Edge Crease Tool | — |
| Add Geometry | Topology Pen | — |
| Add Geometry | Subdivide Polygons | subdivision-2026-11-03, mesh-commands-2026-11-02 |
| Add Geometry | Bevel (picks Polygon/Edge/Vertex Bevel by selection mode) | bevel-activation-2026-11-05, polygon-bevel-2026-11-02, edge-bevel-chamfer-2026-11-02, vertex-tool-options-2026-11-03 |
| Add Geometry | Extrude | polygon-extrude-2026-11-02, extrude-2026-11-05 |
| Add Geometry | Bridge | bridge-2026-11-05 |
| Add Geometry | Polygon Slice | — |
| Add Geometry | Smooth Shift | smooth-shift-thicken-2026-11-05 |
| Add Geometry | Thicken | smooth-shift-thicken-2026-11-05 |
| Edit | Cut Selection | create-panel-2026-11-04 (clipboard) |
| Edit | Copy Selection | create-panel-2026-11-04 (clipboard) |
| Edit | Paste Selection | create-panel-2026-11-04 (clipboard) |
| Edit | Paste Tool | — |

### Deform

| Group | Button | Covered by |
|---|---|---|
| Move Tools | Element Move | tweak-2026-11-03, tweak-range-2026-11-04 |
| Move Tools | Element Push | — |
| Move Tools | Flex | — |
| Move Tools | Soft Move | — |
| Move Tools | Soft Drag | — |
| Move Tools | Shear | — |
| Move Tools > Convolve | Smooth | — |
| Move Tools > Convolve | Jitter | — |
| Move Tools > Convolve | Quantize | — |
| Move Tools > Soft Selection | Soft Selection Move | falloff-2026-11-01 |
| Move Tools > Soft Selection | Soft Selection Rotate | falloff-2026-11-01 |
| Move Tools > Soft Selection | Soft Selection Scale | falloff-2026-11-01 |
| Move Tools > Soft Selection | Soft Selection Transform | falloff-2026-11-01 |
| Rotate Tools | Twist | — |
| Rotate Tools | Bend | — |
| Rotate Tools | Vortex | — |
| Rotate Tools | Swirl | — |
| Scale Tools | Push | — |
| Scale Tools | Sculpt | — |
| Scale Tools | Taper | — |
| Scale Tools > Bulge | Bulge | — |
| Scale Tools > Bulge | Flare | — |
| Align Tools | Radial | — |
| Align Tools | Linear | — |

### Duplicate

| Group | Button | Covered by |
|---|---|---|
| — | Radial Sweep | — |
| Mirror | Mirror | symmetry-2026-11-01 (partly) |
| Mirror | Instance Mirror | — |
| Mirror | Replica Mirror | — |
| — | Bridge | bridge-2026-11-05 |
| — | Patch Curves | — |
| Curve Extrude | Curve Extrude | — |
| Curve Extrude | Bezier Extrude | — |
| Curve Extrude | B-Spline Extrude | — |
| Curve Extrude | Pen Extrude | — |
| Curve Clone | Curve Clone | — |
| Curve Clone | Curve Instance | — |
| Curve Clone | Curve Replica | — |
| Curve Clone | Curve Transform | — |
| Curve Clone | Bezier Clone | — |
| Curve Clone | Bezier Instance | — |
| Curve Clone | Bezier Replica | — |
| Curve Clone | Bezier Transform | — |
| Curve Clone | B-Spline Clone | — |
| Curve Clone | B-Spline Instance | — |
| Curve Clone | B-Spline Replica | — |
| Curve Clone | B-Spline Transform | — |
| Curve Clone | Pen Clone | — |
| Curve Clone | Pen Instance | — |
| Curve Clone | Pen Replicate | — |
| Curve Clone | Transform Pen | — |
| Scatter Clone | Scatter Clone | — |
| Scatter Clone | Instance Scatter | — |
| Scatter Clone | Replica Scatter | — |
| Scatter Clone | Transform Scatter | — |
| Clone | Clone | — |
| Clone | Instance Clone | — |
| Clone | Replica Clone | — |
| Array | Array | — |
| Array | Instance Array | — |
| Array | Replica Array | — |
| Array | Transform Array | — |
| Radial Array | Radial Array | — |
| Radial Array | Instance Radial Array | — |
| Radial Array | Replica Radial Array | — |
| Radial Array | Transform Radial Array | — |
| Mesh Paint | Mesh Paint | — |
| Mesh Paint | Mesh Paint Instance | — |
| Mesh Paint | Mesh Paint Replica | — |
| — | Paste Tool | — |
| — | Duplicate | — |
| — | Duplicate Hierarchy | — |

### Mesh Edit

| Group | Button | Covered by |
|---|---|---|
| Tools | Slice | — |
| Tools | Curve Slice | — |
| Tools | Edge Slice | — |
| Tools | Pen Slice | — |
| Tools | Loop Slice | — |
| Tools | Axis Slice | — |
| Tools | Julienne Slice Tool | — |
| Tools | Tack Tool | — |
| Tools | Bevel | polygon-bevel-2026-11-02, bevel-activation-2026-11-05 |
| Tools | Extrude | polygon-extrude-2026-11-02, extrude-2026-11-05 |
| Commands > Subdivide | SDS Subdivide | subdivision-2026-11-03, mesh-commands-2026-11-02 |
| Commands > Subdivide | Faceted | subdivision-2026-11-03, mesh-commands-2026-11-02 |
| Commands > Subdivide | Smooth | subdivision-2026-11-03, mesh-commands-2026-11-02 |
| Commands > Subdivide | Options | — |
| Commands > Subdivide | UnSubdivide | subdivision-2026-11-03, mesh-commands-2026-11-02 |
| Commands | Boolean CSG | — |
| Commands | Solid Drill | — |
| Commands | Axis Drill | — |
| Commands | Fix Black Dots | — |
| Commands | Edge Zip | — |
| Commands | Diagonal Re-Direct | — |
| Commands | Mesh Cleanup | mesh-cleanup-2026-11-05 |
| Commands | Automatic Retopology | — |
| Commands | Retopo Edge Lock | — |
| Commands | Retopo Edge Flow | — |

### Vertex

| Group | Button | Covered by |
|---|---|---|
| Tools | Vertex Tool | vertex-tool-options-2026-11-03 |
| Tools | Bevel | vertex-tool-options-2026-11-03, bevel-activation-2026-11-05 |
| Tools | Extrude | extrude-2026-11-05, vertex-tool-options-2026-11-03 |
| Tools | Merge Tool | vertex-tool-options-2026-11-03 |
| Tools | Drag Weld | vertex-tool-options-2026-11-03 |
| Tools | Slide | vertex-tool-options-2026-11-03 |
| Commands | New Vertex | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Center | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Center All Axis | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Set Position | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Align | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Remove | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Collapse | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Join | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Join Averaged | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Vertex Tear | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Split | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |
| Commands | Merge | vertex-tool-options-2026-11-03, mesh-commands-2026-11-02, set-position-2026-11-05 |

### Edge

| Group | Button | Covered by |
|---|---|---|
| Tools | Extrude | extrude-2026-11-05, vertex-tool-options-2026-11-03 |
| Tools | Extend | — |
| Tools | Bevel | edge-bevel-chamfer-2026-11-02, bevel-activation-2026-11-05 |
| Tools | Bridge | bridge-2026-11-05 |
| Tools | Add Loop | add-loop-edge-slide-2026-11-02 |
| Tools | Add Point | — |
| Tools | Edge Relax | — |
| Tools | Edge Chamfer | edge-bevel-chamfer-2026-11-02 |
| Commands | Spin | mesh-commands-2026-11-02 |
| Commands | Collapse | mesh-commands-2026-11-02 |
| Commands | Remove | mesh-commands-2026-11-02 |
| Commands | Join | mesh-commands-2026-11-02 |
| Commands | Join Averaged | mesh-commands-2026-11-02 |
| Commands | Split | mesh-commands-2026-11-02 |
| Commands | Grow Quads | — |
| Commands | Fill Quads | — |
| Commands | Split Normals | — |
| Commands | Edges to Curves | — |
| Commands | Unbevel | — |

### Polygon

| Group | Button | Covered by |
|---|---|---|
| Tools | Pen | pen-2026-11-05, create-panel-2026-11-04 |
| Tools | Bevel | polygon-bevel-2026-11-02, bevel-activation-2026-11-05 |
| Tools | Smooth Shift | smooth-shift-thicken-2026-11-05 |
| Tools | Thicken | smooth-shift-thicken-2026-11-05 |
| Tools | Sketch Extrude | sketch-extrude-2026-11-05 (v0) |
| Tools | Bridge | bridge-2026-11-05 |
| Tools | Spikey | spikey-2026-11-05 |
| Tools | Shift | — |
| Tools | Inset | inset-2026-11-05 |
| Tools | Reduction Tool | poly-reduce-2026-11-05 |
| Tools | Topology Pen | — |
| Tools | Symmetrize | — |
| Commands | Make | make-split-2026-11-05 |
| Commands | Split | make-split-2026-11-05 |
| Commands | Triple | mesh-commands-2026-11-02 |
| Commands | Quadruple | quadruple-2026-11-05 |
| Commands | Spin Quad Pairs | mesh-commands-2026-11-02 |
| Commands | Flip Polygons | poly-flip-2026-11-05 |
| Commands | Align Polygons | mesh-commands-2026-11-02 |
| Commands | Polygon Set Part | set-part-2026-11-04, set-part-observed-2026-11-05 |
| Commands | Polygon Set Material | set-material-2026-11-05 |
| Commands | Make Curve Fill | — |
| Commands > Polygon Scale Offset | PolyScale Offset | offset-flatten-2026-11-05 |
| Commands > Polygon Scale Offset | PolyScale Offset Detach | offset-flatten-2026-11-05 |
| Commands | Flatten Ngons | offset-flatten-2026-11-05 |
| Commands > Flip It | Flip It | — |
| Commands > Flip It | Flip It Origin | — |
| Commands | Triple Fan | — |
| Commands > Reduce | Merge | mesh-commands-2026-11-02 |
| Commands > Reduce | Collapse | mesh-commands-2026-11-02 |
| Commands > Reduce | Collapse Selection | — |
| Commands > Reduce | Remove | mesh-commands-2026-11-02 |
| Commands > Reduce | Unify Polygons | mesh-commands-2026-11-02 |
| Commands > Reduce | Detriangulate | — |

### Curve

| Group | Button | Covered by |
|---|---|---|
| Tools | Bezier | — |
| Tools | Curve | — |
| Tools | B-Spline | — |
| Tools | Sketch | — |
| Tools | Retopo Sketch | — |
| Tools | Solid Sketch | — |
| Tools | Tube | — |
| Tools | Patch | — |
| Tools | Hair Guide | — |
| Tools | Set Curves Color | — |
| Commands | Freeze Polygons | — |
| Commands | Make Open Curve | — |
| Commands | Make Closed Curves | — |
| Commands | Edges to Curves | — |
| Commands | Constrain to Background | — |
| Commands > Open Curve Offset | Open Curve Offset | — |
| Commands > Open Curve Offset | Open Curve Offset Trio | — |
| Commands > Closed Curve Offset | Closed Curve Offset | — |
| Commands > Closed Curve Offset | Closed Curve Offset Trio | — |
| Convert Curves | Curve to B-Spline | — |
| Convert Curves | Curve to Bezier | — |
| Convert Curves | Bezier to Curve | — |
| Convert Curves | Bezier to B-Spline | — |
| Convert Curves | B-Spline to Bezier | — |
| Convert Curves | B-Spline to Curve | — |

## Open questions and decisions

- **Q-TM-01 (decision pending)** Which uncovered groups come next. Suggested order by
  everyday use: Polygon > Shift / Triple Fan / Detriangulate; Edge >
  Extend / Add Point / Edge Relax / Grow and Fill Quads; Mesh Edit > Slice
  family and Boolean; then Deform, Duplicate and Curve as their own
  milestones.
- **Q-TM-02** Other toolboxes (Topology, UV, Sculpt) aren't listed; add
  them if Operandi needs them.
