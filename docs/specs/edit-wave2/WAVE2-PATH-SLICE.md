# Wave 2: Curve Slice, Pen Slice, Primitive Slice — behaviour spec v0

Provenance: the reference application's user guide (Curve Slice, Pen Slice,
"Creating Geometry on a Mesh Using Preset Shapes"), its plain-text UI
configuration (which parts make up each tool, preset values), and its live
tool-attribute listing read through the bridge (names and starting values
only; nothing was applied). **No scripted cut has been run yet**: the
v1 update (2026-11-08): 23 scripted cuts run with Jason's OK (Curve Slice
and Primitive Slice on a 4×4 flat grid and a 2 m cube; oracle data in
`cases-wave2.json`). Rules marked **Observed** come from those. Pen Slice
couldn't be driven by script (its points need clicks), so its rules stay
Provisional and follow the observed cutter rules.
REA-assisted: no.

Read with: `toolbox-panels-2026-11-06/EDIT.md` §2 (Slice section; this file
supersedes its Curve, Pen and Primitive Slice paragraphs), Operandi's
slice cut engine (Polygon/Edge/Loop Slice), the Pen spec (`pen-2026-11-05`),
the Create panel spec (curve drawing), the Booleans spec in the Library
(`booleans/BOOLEANS.md`, for the boolean operation types).

Status tags: **Docs**, **Config** (preset or attribute listing),
**Provisional**.

---

## 1. One tool family, three path makers

**PS-01 (Config).** All three tools are the same two parts stacked:

1. a **path maker** that produces a 2D path or outline:
   - Curve Slice: a curve (a background curve, or one drawn now), sampled
     into straight steps by **Path Steps**;
   - Pen Slice: a pen path (clicked points, straight segments);
   - Primitive Slice: a shape (rectangle, circle, ellipse, or a profile
     preset), placed on a surface;
2. a **Slice cutter** that projects that path along an axis onto the mesh
   and cuts it.

Operandi should build the cutter once (PS-10..PS-17) and give each tool
its own path maker. The cutter extends the existing slice cut engine
(points on edges, points inside polygons, chords).

**PS-02 Panel (Docs/screenshot).** Curve Slice sits second and Pen Slice
fourth in the Slice section, Primitive Slice last (order: Polygon, Curve,
Edge, Pen, Loop, Primitive). Large icon + label like the other Slice
buttons; Operandi draws its own icons.

## 2. The Slice cutter (shared)

Options (labels ours where the reference's are unclear), with starting
values from the attribute listing:

| Option | Values | Starts at |
|---|---|---|
| Operation | Core, Tunnel, Stencil, Slice, Union, Intersect, Subtract, Add | **Slice** |
| Axis | X, Y, Z, Custom, Action Axis | **Y** (Curve, Pen); **Action Axis** (Primitive) |
| Vector X/Y/Z | direction for Custom | 0, 1, 0 |
| Front Face Only | on/off | off |
| Material | material name (Stencil) | "Default" |
| Negative Depth / Positive Depth | distances, boolean types only | 0 / 1 m |
| Preview Geometry | on/off, boolean types only | on |
| Select Sliced Edges | on/off | off |
| Edge Selection Set | name | "Sliced Edges" |

**PS-10 Projection (Observed).** The path is swept along the Axis into an
endless ribbon (a curtain standing along the axis); every polygon of the
active mesh that the ribbon crosses is cut along the crossing line. So a
path drawn above a box cuts its top and bottom, and also its side faces
wherever the ribbon passes through them (a straight line across a cube
splits the two side faces it crosses from top to bottom). Polygons lying
in the ribbon's own plane are not cut. No existing point moves.

- **Points:** a new point wherever the ribbon crosses a polygon edge, and a
  new point inside a polygon for every path vertex (pen point, curve step
  point, shape corner) that lands inside it, placed on that polygon. Path
  vertices outside the mesh are ignored. Path vertices that lie on a
  straight run are still kept (a straight two-point curve sampled in 24
  steps leaves every step point on the cut).
- **Split:** each crossed polygon is split along the chain of new points
  from one edge crossing to the next.
- **Through an existing point:** when the path passes exactly through an
  existing mesh point, the reference adds a second point at the same spot
  instead of reusing it. Operandi reuses the existing point (deliberate
  difference; a coincident duplicate is a defect, not a feature).

**PS-11 Front Face Only (Observed).** Off: every polygon the ribbon
crosses is cut (top, bottom and sides of a box). On: only polygons whose
normal points along the axis direction (positive dot product with the
axis vector; for Axis Y, upward-facing polygons) are cut; a cube's top was
cut, its bottom and sides were not.

**PS-12 Open ends (Observed).** An open path that ends inside a polygon
cuts into it as a **slit**: the polygon is not split; its outline runs in
along the cut to the path's end and back out again (a zero-width notch
through the new points). Polygons the path fully crosses are split as
usual. Operandi copies the slit if its mesh allows it; otherwise
(Provisional) it keeps the new points on that polygon's crossed edge and
skips the slit, and says so in the build notes.

**PS-13 Closed path inside one polygon (Observed).** A closed outline
lying wholly inside one polygon makes the outline a new polygon, and the
outer polygon keeps a hole bridged to its outline by one seam: its point
list runs round the outside, along the seam to the hole, round the hole the
other way, and back along the seam. The seam joins an outer corner to a
nearby hole point (both cases measured had several equally short
choices; tie rule open, PS-Q3).

**PS-14 Operations (Observed for Slice, Core, Tunnel; Docs for Stencil).**
Each applies to every surface the ribbon crosses (Front Face Only limits
it).

- **Slice**: cut the outline in, nothing removed, materials unchanged.
- **Stencil**: same cut as Slice (same points and polygons, Observed), and
  polygons inside the outline get **Material**.
- **Core**: keep only the polygons inside the outline; everything outside
  it is deleted, together with points no longer used (a rectangle on a
  flat grid left just the four pieces inside it; on a cube, the two
  rectangles on top and bottom, nothing else).
- **Tunnel**: delete the polygons inside the outline on every crossed
  surface, leaving holes; **no side walls are made** (a cube cut top and
  bottom kept its six outer faces with keyhole holes in top and bottom,
  and nothing joins the two holes). On a single surface it simply cuts the
  inside away.
- **Union, Intersect, Subtract, Add**: boolean operations with the outline
  pushed into a solid by Negative/Positive Depth. **Wave 4**; greyed until
  then. (Depth, Material and Preview fields are disabled for operations
  they don't apply to: Depth for Intersect/Subtract only, Material for
  Stencil only.)

**PS-15 Select Sliced Edges (Docs).** On: the new cut edges are added to an
edge selection set with the given name.

**PS-16 Undo.** One undo step per apply (the tool stays live, each change
re-applies like other Operandi tools).

**PS-17 Selection after (Observed for no selection).** With nothing
selected beforehand, nothing is selected afterwards. With a selection:
Provisional, pieces of selected polygons stay selected.

## 3. Curve Slice

**PS-20 Source (Docs).** On activation: if a curve sits in a visible,
unselected background mesh, it is the cutter. Otherwise the user draws one
in the viewport (click to add points) and edits it like any Operandi
curve (Create panel curve drawing). Operandi has no background meshes yet:
build the draw mode first; background curves come with background meshes.

**PS-21 Path Steps (Observed).** The curve passes through its points and
is sampled at **Steps** points evenly spaced in curve parameter (Steps 24
on a two-point line: 24 equal intervals; Steps 4 on a three-point curve:
the two ends, the middle point and one point halfway along each half).
Use Operandi's existing curve evaluation for the shape. Options: **Steps**
(starts 24), **Auto** (off; picks the number by angle), **Smooth angle**
(about 5°, used by Auto). A two-point curve is a straight line, but its
step points are still added (PS-10).

**PS-22 Options (Config).** Curve: **Closed** (off), plus the cutter's
options. Starting Axis Y: so in a front view the user must change Axis to
Z to cut what they drew (Docs leave this to the user). Provisional for
Operandi: same starting values, no auto axis.

## 4. Pen Slice

**PS-30 (Docs/Config).** Draw a path exactly like the Pen tool: click
points on or over the mesh; the Pen's self-snaps (**World Axis**,
**Straight Line**, **Right Angle**, all on) apply. **Type**: **Closed**
(joins the last point to the first; the starting value) or **Open**.
Straight segments between points. Cutter Axis starts at Y.

**PS-31 (Provisional; not scriptable).** Points are placed on the work plane / surface as
the Pen tool does (Operandi's Pen rules); the cut is the projection of the
segments along the Axis.

## 5. Primitive Slice

**PS-40 Shape (Docs/Config; Observed for Rectangle and Circle).**
(Corrected 2026-11-08, W2-PS-Q14.) The shape has its own frame: local Z
is the cutter axis; **Half Size X runs along local X, Half Size Y along
local Y**. For Axis Y that puts **Half Size X along world Z and Half Size Y
along world X** (a rectangle centred at (0.3, 0, 0.2) with Half Size
0.7 × 0.4 spans x −0.1…0.7 and z −0.5…0.9). Local X, local Y, axis form a
right-handed frame (for Axis Y: Z, X, Y). A circle's corners lie on the
circle of Radius, every 360°/Sides, with one corner on a local axis.
**Type**: Rectangle (starts), Circle,
Ellipse, Preset (a profile from the preset browser). Rectangle: **Half Size
X/Y** (0.5, 0.5), **Corner Radius** (0) with **Corner Segments** (8) and
**Exponent** (0) for rounded rectangles. Circle: **Radius** (0.5),
**Sides** (24). Ellipse: Half Size X/Y + Sides. Preset: any closed or open
profile; **Extend Open Edge** (0) lengthens open profiles' ends.

**PS-41 Placing (Docs).** Click on a mesh face: the shape is centred there,
lying on that face (Align on: oriented to the face; Provisional: the cutter
axis then follows the face normal, which is what "Action Axis" gives for a
clicked face). Drag to size it (box corners or handles). Clicking another
face moves it there. The shape stays live and editable until the tool is
dropped. **Edit From Center** (off; or hold Ctrl/Cmd while drawing): the
click point is the centre instead of a corner. Right-drag up/down resizes
from the centre (Provisional: bind to Operandi's equivalent haul).

**PS-42 Rotating (Docs; directions Observed 2026-11-08, W2-PS-Q15).**
**Show Rotate Handles** (off) shows ring handles; or type the angles:

- **Bank** turns the shape in its own plane, about the cutter axis,
  right-handed: +30° on Axis Y is counter-clockwise seen from +Y (corner
  (x 0.4, z 0.7) goes to (0.696, 0.406)).
- **Heading** tilts the shape about its local X (the Half Size X
  direction): its Half Size Y extent shrinks by cos(angle) in the cut
  (30° on a 0.7 × 0.4 rectangle: x half-extent 0.4 → 0.346, z unchanged).
- **Pitch** tilts it about its local Y (the Half Size Y direction): its
  Half Size X extent shrinks by cos(angle) (30°: z half-extent 0.7 →
  0.606, x unchanged).
- The sign of Heading and Pitch can't be seen with a centred symmetric
  shape (its projection is the same either way). Provisional:
  right-handed about the local axis, like Bank (PS-Q10). Order when several
  are set: Provisional Heading, then Pitch, then Bank.

**Keep Aspect** (off) keeps the width-to-height ratio while sizing.
**Flip X / Flip Y** mirror the shape.

**PS-43 Clones (Docs).** **Clone X / Clone Y** (1, 1) repeat the shape in a
grid, **Clone Gap X / Y** (0.01) apart; **Center** recentres the cloned
group on the click point.

**PS-44 Lazy Apply (Docs).** On (starts on): while dragging, only a box
shows the shape's size; the cut happens when the mouse is released. Off:
cut live while dragging.

**PS-45 Operations.** Build Slice, Stencil, Tunnel and Core in wave 2;
Union/Intersect/Subtract/Add in wave 4.

## 5a. Builder deviations (recorded 2026-11-08; Jason to confirm)

The wave 2 builder keeps these differences from the reference; they are
listed so the comparison step doesn't report them as bugs:

- No slits (PS-12): an open path ending inside a polygon doesn't notch it.
- A closed outline inside one polygon (PS-13) comes out as two pieces
  (inner polygon plus an outer polygon split so no seam is needed) rather
  than one keyhole polygon.
- No T-junctions: neighbouring polygons get the new edge points too.
- Bent curves use Operandi's own curve shape, so step points on curved
  stretches won't match the reference exactly (straight paths still do).

## 6. What to build first

1. The cutter with Slice operation, Axis X/Y/Z/Custom, Front Face Only
   (PS-10..PS-13), on Operandi's cut engine.
2. Pen Slice (reuses Pen drawing; straight segments, simplest path).
3. Primitive Slice with Rectangle and Circle, placing/sizing, Lazy Apply;
   then Ellipse, rounded rectangles, rotate, clones; then Stencil, Tunnel,
   Core.
4. Curve Slice draw mode with Path Steps; background curves later.
5. Un-grey Loop Slice's Keep Quads and Split/Cap/Gap if time allows
   (separate, small).

## 7. Open questions (planned probe batch)

- **PS-Q1** Inner points on non-planar polygons; points landing very near
  an existing point or edge.
- **PS-Q3** Hole seam: which outer corner and hole point when several are
  equally close.
- **PS-Q7** Which polygons are cut when some are selected.
- **PS-Q8** Selection after the cut when polygons were selected.
- **PS-Q10** Heading/Pitch sign and rotation order (needs an off-centre shape).
- **PS-Q9** Pen Slice: anything that differs from the cutter rules
  (needs a hands-on check; its points can't be scripted).
