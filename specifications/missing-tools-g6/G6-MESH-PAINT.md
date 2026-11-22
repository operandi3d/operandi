# Missing tools, group 6: Mesh Paint and Mesh Paint Instance (v1, 2026-11-21)
Behaviour only. No stroke was recorded numerically, so placement details marked (reading) are Operandi's reading of the guide. REA-assisted: no.
What it does: a brush that places copies of a source mesh onto a target surface where you click or paint: one at a time, scattered along a stroke, or as a line, grid or ring. Mesh Paint makes real geometry copies; Mesh Paint Instance makes mesh instances instead. (Mesh Paint Replica out of scope: needs particles.) Placer x copier pair (TOOL-FAMILIES §8): the surface brush placer is new; the copiers exist (Clone's copy-geometry effector; Instance Clone's instance effector).

Setup
- MP-01 The selected (active) mesh is the target and receives the copies. The source is a visible background mesh (MP-14). Needs at least one background mesh; with none: "Mesh Paint needs a background mesh to copy." (reading)
- MP-02 Clicks and strokes hit the target's surface (the selected mesh; a Target Mesh setting can name another). Missing the surface places nothing.

Settings (defaults)
Paint Mode = Slide (Slide, Strokes, Drop, Drop (no Scale), Linear, Array, Radial)
Brush Size = 20 px (screen radius, Strokes area)
Brush Density = 50 % (copies per brush area in a Strokes pass)
Size Mode = Locked (Uniform, Adaptive, Random, Adaptive + Random, Locked)
Size X/Y/Z = 100 % (multipliers; Random: the variation; 0 % = none)
Scale Increment = 0 % (step for interactive scaling; 0 = free)
Rotation Mode = Surface Align (Uniform, Surface Align, Random, Surface Align + Random, Surface Align + Tablet, Surface Align + Tablet (no Pitch), Screen Align + Tablet, Screen Align + Tablet (no Pitch))
Rotation X/Y/Z = 0° (offsets; Random: the variation)
Rotation Snapping / Angle = off / 15°
Offset = 0 m (lift along surface normal)
Parameter 1 -> Effect = Falloff -> Density; Parameter 2 -> Effect = Pressure -> Density. Parameter: Falloff, Pressure, Slope, Altitude, 1.0 − Slope, 1.0 − Altitude. Effect: (none), Density, Size, Rotation, Size + Density
Linear Count = 2; Array Count X/Y = 2/2; Radial Count = 6
3D Brush = off (brush is a sphere in 3D instead of a screen circle; reading)
Geometry Source = Random BG Layer (All BG Layers, Random BG Layer)
Create Instances = on (Instance variant)
Brush falloff: the Smooth brush shape (Size, In/Out 0.5/0.5, Bias 50 %, Auto Scale off) from Operandi's existing brush engine; feeds Parameter "Falloff" (centre 1, edge 0).

Paint modes
- MP-03 Slide: press on the surface: one copy appears under the cursor; drag to slide it over the surface; release places it. Ctrl held before the click: drag sets its size instead.
- MP-04 Strokes: copies scattered at random inside the brush circle along the stroke. Count: Brush Density x brush area, scaled by any Density effect (falloff makes centre denser). (reading; smooth, seedable random so undo/redo and reload give the same result)
- MP-05 Drop: click places one copy; dragging from it scales (by drag length) and rotates it about the surface normal (by drag direction). Drop (no Scale): only rotates.
- MP-06 Linear: drag A to B: Linear Count copies evenly from A to B along the surface. Shift: 45° screen steps.
- MP-07 Array: drag a rectangle: Array Count X x Y copies on its grid. Shift: square.
- MP-08 Radial: click a centre, drag out: Radial Count copies on a circle on the surface, first at the drag point. Shift: start angle snaps to 45°.

Placement
- MP-09 Position: hit point on the target surface, moved Offset along the surface normal.
- MP-10 Orientation: Uniform keeps the source's own orientation (+ X/Y/Z offsets); Surface Align turns the source's +Y onto the surface normal (heading: stroke direction for Strokes/Linear, otherwise unchanged; reading); Random adds random rotation up to X/Y/Z; Tablet modes take pen tilt (fall back to Surface Align with no tilt; Screen Align = view's up instead of surface normal).
- MP-11 Size: Uniform = source size x Size X/Y/Z (source item's own transform ignored); Adaptive = follows brush size on screen; Random = random variation up to Size X/Y/Z; Locked = fixed world size that doesn't change with zoom.
- MP-12 Parameter value (0–1) multiplies the chosen effect. Slope = 0 flat … 1 vertical; Altitude = height in target's bounds (0 bottom … 1 top); Pressure = pen pressure (1 with a mouse).
- MP-13 The source's pivot (origin) sits on the hit point.

Copiers
- MP-14 Mesh Paint (geometry): each placement adds the source's polygons to the target mesh as real geometry. Random BG Layer (default) picks one visible background mesh at random per copy; All BG Layers uses all visible background meshes merged as one. Clone options as the existing Clone copier: Replace Source off, Invert Polygons off, Merge Vertices off (Distance 0).
- MP-15 Mesh Paint Instance: each placement adds a mesh instance of the source as a new item. Parent: Off (default; top-level items), Hierarchy, Root (children of the source), Primary (children of a group locator under the source). Bounding Box display off. Hierarchy (copy child items) off.
- MP-16 One undo step per stroke / click.
- MP-17 Symmetry on: each placement mirrored too. Snapping to Vertex / Polygon Center applies to Slide and Drop positions.

Where it lives
- MP-18 Duplicate tab and Create tab: a Mesh Paint button with the Instance variant (two buttons, or one with an Alt/Shift variant as Operandi's other duplicate buttons do).

Open: Q-MP-1 Strokes count/spacing and Surface Align heading are readings; build as written, flag anything odd. Q-MP-2 Replica and Create Particle Mesh excluded.
