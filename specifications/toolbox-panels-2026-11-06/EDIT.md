# Edit panel — behaviour spec v0 (docs + UI config)

Provenance: the reference application's offline user guide (Slice tools,
Loop Slice, Edge Slice, Axis Slice, Julienne, Primitive Slice, Center,
Subdivide and UnSubdivide, Booleans, Solid Drill, Axis Drill, Curve
Boolean, Fix Black Dots, Edge Zip, Automatic Retopology, Conditional
Commands, Slide) for names and intent; its plain-text toolbox, preset and
command-help config for button order, modifier variants and starting
values. Nothing observed yet. REA-assisted: no.

Read with: the tool audit (Slide, Loop Slice notes), the subdivision spec
(SDS, Freeze), the mesh commands spec (Subdivide), the default shortcuts
spec (G = Edge Slice), the Pen spec (Pen Slice), the Create panel spec
(curves, primitives).

---

## 1. Panel layout

A vertical tab named **Edit**, between Duplicate and Vertex. Shared header
(transform icon row, **Mesh Cleanup...**, **New Mesh**; both in Operandi).

| Section | Buttons, top to bottom (shortcut) | Style |
|---|---|---|
| *(top, no title)* | **Slide** | Large; greyed when no component selection mode applies |
| **Slice** | Polygon Slice (Shift+C), Curve Slice, Edge Slice (G), Pen Slice, Loop Slice (Alt+C), Primitive Slice | Large icon + label |
| **Axis Slice** | Polygon Axis Slice, Dicer, Julienne | Large icon + label |
| **Center** | X, Y, Z, All, in one row | Four small text buttons. **Alt+click** X/Y/Z = XY, YZ, ZX |
| **Subdivide** | SDS Subdivide, Faceted, Smooth, Options... (Shift+D), UnSubdivide | Small icon + label. **Alt+click** SDS Subdivide = subdivide twice |
| *(divider)* | Freeze... | Small text button |
| **Booleans & Drill** | Boolean..., Solid Drill..., Axis Drill..., Curve Boolean... | Small icon + label (Curve Boolean text only) |
| **Commands** | Fix Black Dots, Edge Zip, Diagonal Re-Direct | Small icon + label |
| *(divider)* | Automatic Retopology, Retopo Edge Lock, Retopo Edge Flow | Small text buttons |
| *(bottom)* | Conditional Commands... | Full-width small button with a menu |

"..." buttons open a dialog first. Plain D (polygon mode) also runs
Subdivide with the last settings (guide).

---

## 2. Tools

Status: **Exists / Partial / New / Check** as in DEFORM.md.

### Slide — *Exists (as Edge Slide and Vertex Slide), partial. Tier 1.*
One button, one tool whose handles adapt to the selection mode: edges
slide as a loop (Radial handles by default), vertices slide along their
edges (Linear + Face handles), polygons slide too. Operandi has two
separate tools; this button should start whichever matches the mode.
Option gaps are already listed in the tool audit, item 2 (Loop Slide,
Duplicate, Preserve Curvature, Slip UVs, Merge Vertices on by default...).

### Slice section

**Polygon Slice** (Shift+C) — *New. Tier 1.* Cuts the selected polygons (all
if none selected) with a **plane**. Press in the view to set **Start**,
drag to **End**: the plane runs through that line, standing perpendicular
to the work plane. New edges appear live. While active: drag either end
handle or the line between them to move the plane; **middle-click**
somewhere else to make another cut with the same length and direction
there; **Shift+drag** starts a fresh cut, keeping the previous one.
Options: **Start X/Y/Z, End X/Y/Z**, **Fast Slice** (only cut on mouse
release; for dense meshes), **Angle Snap** with **Angle** (with snapping
on, the line snaps to multiples of Angle), **Split** (separate the two
sides into disconnected pieces), **Cap Sections** (with Split: close
enclosed volumes with a cap polygon), **Gap** (with Split: thickness
removed along the cut), **Offset Side** (gap above, below or centred),
**Infinite** (off: only polygons the drawn line segment crosses on screen
are cut; on: the plane extends forever). Result: every polygon the plane
crosses is split along the crossing line; new points sit exactly where
the plane meets each edge.

**Curve Slice** — *New. Tier 2.* Cuts the mesh along a curve projected onto
it. If a curve sits in a visible background mesh, that curve is the
cutter; otherwise draw one with clicks while the tool is active and edit
it like a normal curve. Steps along the curve (Path Steps options, as in
Curve Extrude) set how smooth the cut is. Options: **Axis** / **Vector
X/Y/Z** (projection direction), Select Sliced Edges. Reuse the Create
panel's curve drawing.

**Edge Slice** (G) — *New. Tier 1.* Click from edge to edge to cut a path
across polygons. Press on an edge and drag along it to place the cut
point; already-made cut points can be dragged later. Clicking the first
edge again closes the loop (the two points turn a different colour when
they overlap, then merge). Options: **Split at Middle** (every cut lands at
the edge's middle; **Ctrl** does it for one click), **Split Polygons** (on:
connect the cut points with new edges; off: only add points), **Show**
None / Position (percent) / Distance (to both ends), **Offset** (for cuts
into a polygon's interior), **Snap Value** (round positions to steps; 0% =
free), **Inside Polygons** (click inside a polygon to cut into its
interior), **Always Raycast** (only front-facing polygons in wireframe),
**Multi Slice** (draw a dashed line from the last cut to the pointer).

**Pen Slice** — *New. Tier 2.* Cut with a path drawn exactly like the Pen
tool (click points on the surface). **Type** Open or Closed (Closed joins
the last point to the first). The Pen's self-snaps (World Axis, Straight
Line, Right Angle) apply. Reuse Operandi's Pen drawing.

**Loop Slice** (Alt+C) — *New. Tier 1, highest-value tool on this panel.*
Select two neighbouring polygons or an edge between them to define a
**ring**; the tool cuts new edge loops across that ring. A **slider bar**
appears at the top of the view: drag the small triangle to move the
current slice; drag the bar's end dots to change its length. Options:
**Edit** (what the pointer does: Move a slice, Add a slice by clicking the
bar, Remove one), **Mode** (Free: any position; Uniform: evenly spaced;
Symmetry: mirrored about 50%), **Current** (which slice is active,
numbered left to right), **Count** (number of slices), a **Uniform**
button (re-space free slices evenly), **Position** (percent across the
ring, 50% = middle), **Length** and **Slider X/Y** (the bar's size and
position on screen), **Slice Selected** (cut only within the selected
polygons instead of running the whole ring; respects corners), **Keep
Quads** (add edges so only quads result), **Slice N-gon** (continue
through polygons with more than four sides), **Preserve Curvature** with
**Tension** (100%: new points follow a curve through the surface; 0% =
straight; negative pulls in, larger pushes out), **Select New Polygon**,
**Split / Cap Sections / Gap** (as Polygon Slice), profile options
(Inset, Reverse Direction, Reverse Inset, Keep Aspect, a preset profile
"router" shape). A one-slice Uniform cut ignored an active falloff in the
falloff spec (FAL-B3-05).

**Primitive Slice** — *New. Tier 2.* Click on a mesh face and drag out a
2D shape that is projected onto the surface and used as a cutter. **Type**:
Rectangle, Circle, Ellipse, Preset (any profile). Rectangle has **Corner
Radius** and **Exponent** for rounded rectangles. The shape stays live
until the tool drops: drag to scale (box corners or handles), click
another face to move it there, **Show Rotate Handles** or Pitch/Bank/
Heading to rotate, **Keep Aspect**, **Edit From Center** (or Ctrl while
drawing: grow from the click point), **Half Size X/Y**. Cut **types**:
Core (keep only inside), Stencil (cut outline and give the inside a
material), Tunnel (hole straight through, both sides), Slice (cut outline
only), Union, Intersect, Subtract, Add (boolean-style with an extruded
version of the shape). **Clone X/Y** with **Clone Gaps** repeats the shape
in a grid. **Extend Open Edge** for open profiles. **Lazy Apply** (show a
box while dragging, cut on release). Build Slice/Stencil/Tunnel first;
the boolean types wait for Booleans.

### Axis Slice section

**Polygon Axis Slice** — *New. Tier 2.* Cuts the selected polygons with
evenly spaced planes perpendicular to X, Y and/or Z, spread over the
selection's bounding box. **Slice Mode** Number (**Number X/Y/Z** cuts per
axis across the box) or Size (**Size X/Y/Z** spacing between cuts),
**Offset X/Y/Z** (slide the set of cuts; drag in the view), **Hold
Slice** (pause cutting while adjusting). Polygon mode, selection first.

**Julienne** — *New. Tier 3.* A row of parallel cutting planes that
actually **cuts the mesh into separate closed pieces** (inner faces are
made). First click sets the origin on the work plane, dragging sets the
offset; off-handle drags change the angle. Options: **Number of Clones**
(planes; starts 8), **Offset X/Y/Z** (starts 0.2, 0.2, 0: spacing, or the
last plane with **Between**), **Size X/Y/Z**, **Rotate X/Y/Z** (plane
orientation), **Angle Snap** (starts on, 45°), plus cutter options:
**Preview Only** (on while editing: draw the cuts without making them;
must be off to really cut), **Gap** (0.001: space between pieces), **Inner
Material** (tag for the new inside faces). Needs solid booleans.

**Dicer** — *New. Tier 3.* Same cutter with **random** planes instead of a
row: smashes the mesh into chunks. **Random Seed** (40), **Randomness**,
**Gap** (0.001), **Inner Material**, **Max. Cutting Planes**, **Fragment
Scale X/Y/Z** (long thin vs. cubic chunks), Preview Only. Needs solid
booleans.

### Center section — *New (simple). Tier 1.*
Commands, no dialog. Move the selected geometry so its centre sits at 0 on
the chosen axis: **X**, **Y**, **Z**, or **All** three; **Alt+click** for
**XY**, **YZ**, **ZX**. One vertex: that vertex goes to 0. One edge: its
midpoint. One polygon: its centre. Several elements: the centre of the
selection (bounding box vs. average: CN-Q1). Works on components; in Item
mode use the item's own reset instead (CN-Q2: what Operandi does in Item
mode; sane = nothing, with a status message). One undo step. Also
wanted on the Vertex tab per the tool audit, item 9.

### Subdivide section — *Check (Subdivide exists in Operandi), Tier 1 to wire up.*
- **SDS Subdivide**: subdivide once and move every point (old and new)
  toward the smooth limit surface (like turning on subdivision level 1
  and freezing). **Alt+click**: twice.
- **Faceted**: split each polygon into four (quads) without moving
  anything; shape unchanged.
- **Smooth**: split and nudge new points to round the surface, except
  across edges sharper than **Max Smoothing Angle**.
- **Options...** (Shift+D): dialog with **Subdivision Method** (Faceted,
  Smooth, SDS Subdivide, Catmull-Clark, Fractal: adds random jitter),
  **Max Smoothing Angle**, **Linear UVs** (keep UVs straight instead of
  smoothing them), **Boundary Rules** (Smooth All, Crease All, Crease
  Edges), **Background Constraint** (stick the result onto a background
  mesh).
- **UnSubdivide**: *New, Tier 2.* Reverses a Catmull-Clark subdivision:
  finds groups of four quads, merges them back and restores the corner
  positions. **Iterations**, **Flat** (don't move points), corner hints
  (UV, Material, Part, Smoothing Group, Selection Set) to find the
  starting quads. Gaps may remain where quads don't group cleanly.

Operandi's existing Subdivide command (mesh commands spec) likely covers
most of this; the work is buttons plus the missing methods.

### Freeze... — *Check (subdivision Freeze exists), Tier 2.*
Dialog. Turns curves, patches and subdivision surfaces into plain
polygons. Options seen in the config: **Freeze Curves as** (faces or
other polygon type), **Make Holes** with **Drill Axis**, **Freeze
Displacement**, **Freeze Deformation**, **Tessellation**, **Subdivide
Curves with Fixed Spans**, **Curve Refinement Angle**, **Save Deformation
to Morph** with **Morph Map Name**. One undo step. Subdivision spec SDV-15:
Freeze keeps UVs.

### Booleans & Drill — *New. Tier 2 (Jason's next big item after this).*
All four follow one rule: the **foreground** mesh (selected) is changed,
the **background** mesh (visible, not selected) is the cutter, unless
**Drive Mesh** says Last Selected (cut within one mesh) or a Specific
Surface Item (or group).

- **Boolean...**: dialog, **Operation** Union (merge, remove insides),
  Intersect (keep only the overlap), Subtract (cut the cutter's shape out
  and close the hole with new faces), Add (keep everything, stitch the
  two surfaces where they cross). Meshes must be closed, without
  overlapping duplicate geometry or loose points (suggest Mesh Cleanup
  first). Results are n-gon heavy; fine for hard surface, poor for SubD.
- **Solid Drill...**: a 3D cutter acting on flat polygons. **Operation**
  Core (keep inside the outline), Stencil (cut outline, new material
  inside, named by **Material**), Tunnel (hole), Slice (cut outline only).
- **Axis Drill...**: the same four operations with a **2D** cutter
  projected along an **Axis**.
- **Curve Boolean...**: booleans between flat **curves** (2D shapes):
  Slice (add points where curves cross), Union, Intersect, Subtract;
  **Axis** (Auto fits the plane), **Driver Curve** (unselected curves in the
  same mesh, or a driver mesh), **Clip**, **Merge Curves**, **Close Open
  Curves**, **Trim Open Curves**, **Self Intersection**, optional
  **Constraint Target** with **Offset from Target** (wrap curves onto a
  surface).

Each is one undo step. A separate Boolean spec is worth doing before the
build: edge cases matter more than options here.

### Commands

**Fix Black Dots** — *New. Tier 3.* One-click clean-up that removes pinches
and tiny holes in the mesh's topology that show as dark dots in shaded
views. Exact rule unknown (FB-Q1); probe before building.

**Edge Zip** — *New. Tier 3.* Select an edge loop (e.g. a diagonal loop on
a sphere); the loop is removed and the mesh's edges on either side are
welded together, reworking SubD topology. Probe (EZ-Q1).

**Diagonal Re-Direct** — *New. Tier 3.* No guide page. Name suggests it
turns diagonal edge flow into a different direction. Probe (DR-Q1).

**Automatic Retopology** — *New. Tier 3, own milestone.* Builds a new
clean mesh of evenly sized tris and quads over the input. Options include
**Poly Count Mode** (Relative, Absolute, Adaptive), **Relative Polygon
Target Count**, **Polygon Target**, a crease angle for sharp edges, and
guides: a density weight map, plus edges tagged **Retopo Edge Lock**
(points must lie on them) and **Retopo Edge Flow** (edges should follow
them). Big algorithm; not for this round.

**Retopo Edge Lock / Retopo Edge Flow** — *New, ships with Auto Retopo.*
Add the selected edges to a named edge selection set ("Retopo Edge Lock"
or "Retopo Edge Flow"). Trivial once selection sets exist.

**Conditional Commands...** — *New. Tier 3.* A user setup screen: one
custom key runs different commands depending on the selection mode and
how many elements are selected (None, One, Two+, Default per Vertex /
Edge / Polygon / Item), with sticky or toggle key behaviour. Operandi's
keybinding editor is the natural home for this. Later.

---

## 3. Open questions

- **PS-Q1** Polygon Slice with Infinite off: exactly which polygons are
  cut (screen-space segment crossing, or 3D segment)?
- **LS-Q1** Loop Slice starting values (Count, Mode, Position) and what
  the bar does on first activation.
- **ES-Q1** Edge Slice: does a cut that crosses non-quads behave the same?
- **CN-Q1** Center: bounding-box centre or average of points?
- **CN-Q2** Center in Item mode.
- **FB-Q1, EZ-Q1, DR-Q1** Fix Black Dots, Edge Zip, Diagonal Re-Direct
  behaviour (bundled scripts, no guide detail).
- **BO-Q1** Boolean edge cases: coplanar faces, open meshes, multiple
  background meshes.
- **ED-Q1** Starting values for all tools not in the config (a scripted
  dump, one pass).
