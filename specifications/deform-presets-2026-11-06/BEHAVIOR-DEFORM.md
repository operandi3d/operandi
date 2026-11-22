# Deform panel — behaviour spec v0 (docs + UI config)

Provenance: the reference application's offline user guide (Deform tools
pages, falloff pages, sculpt utility commands) for names and intent; its
plain-text toolbox and tool-preset config for button order, flyouts and
starting values. Nothing observed by script or by hand yet. REA-assisted: no.

Read with: the falloff spec (Linear and Radial weights, shapes, Auto Size,
Reverse, Symmetric), the Tweak spec (Element Move), the mid-tool undo rule,
and the Create panel spec (Quick Align, Ground Align).

**The big idea.** Almost every Deform button is not a new tool. It is a
**preset**: one of the existing transforms (Move, Rotate, Scale, Push)
switched on together with one falloff and sometimes a fixed action center.
Operandi already has the transforms and the Linear and Radial falloffs, so
most of this panel is wiring presets, plus three new falloff types
(Cylinder, Screen, Selection) and four genuinely new tools (Bend, Smooth,
Jitter, the two Align tools).

---

## 1. Panel layout

A vertical tab named **Deform**, between Select and Duplicate. Large icon
plus label buttons, five collapsible sections, all open at start.

| Section | Buttons, top to bottom (shortcut) | Flyout (▸ = small corner triangle; press and hold) |
|---|---|---|
| **Move Tools** | Element Move (T), Element Push, Flex (Shift+Y), Soft Drag ▸, Smooth ▸, Soft Selection Move ▸ | Soft Drag ▸ Soft Drag, Soft Move, Shear. Smooth ▸ Smooth, Jitter, Quantize. Soft Selection Move ▸ Soft Selection Move, Rotate, Scale, Transform |
| **Rotate Tools** | Twist, Bend (Ctrl+E), Vortex, Swirl | — |
| **Scale Tools** | Push, Sculpt, Taper, Bulge ▸ | Bulge ▸ Bulge, Flare |
| **Align Tools** | Radial, Linear (both greyed until there is a usable selection); then one half-width row: Quick Align, Ground Align | — |
| **Commands** | Relax Mesh... (small text button, opens a dialog) | — |

A flyout button shows the last tool picked from it (same behaviour as the
existing flyouts). The Quick Align / Ground Align row is the same control
as on the Create tab; reuse it, don't duplicate the code.

---

## 2. Shared behaviour for every preset tool

These rules apply to Soft Move, Soft Drag, Shear, Twist, Vortex, Swirl,
Taper, Bulge, Flare and the Soft Selection family.

- **Activation.** The button switches the transform and its falloff on
  together. Nothing changes until the first click in a viewport.
- **First click auto-fits the falloff.** On the first press in the view,
  the falloff is placed and sized to the selection's bounding box (whole
  mesh if nothing is selected), using the falloff spec's Auto Size rules.
  For Linear-falloff presets the line runs **across the screen direction
  the pointer is in** (roughly perpendicular to the pointer); for Radial
  ones the centre sits where the pointer meets the work plane. Dragging
  straight away then applies the transform.
- **Right-drag redefines the falloff.** Right-press away from the handles
  and drag to draw a new falloff (a line for Linear, a circle or ellipse
  for Radial/Cylinder, a screen circle for Screen). Left-drag then
  transforms.
- **Falloff handles stay editable.** Start/End, Centre/Size handles can
  be dragged after the auto-fit.
- **Properties panel** shows the transform's fields (Position, Angle or
  Scale) and below them the falloff's fields (Start/End or Centre/Size,
  Auto Size, Reverse, Symmetric, Shape Preset, In/Out, Mix Mode), plus
  Slip UVs and Morph where the transform has them.
- **Weights.** Every point gets its falloff weight w (0..1) and receives
  w of the transform, exactly as the falloff spec says. Topology never
  changes.
- **Undo.** Follows Operandi's mid-tool undo rule: each press-to-release
  or field edit is one step while the tool is active; after the tool is
  dropped, one undo removes the whole action.
- **Drop.** Space or picking another tool drops it; the falloff turns off
  with the preset (it doesn't stay on in the top bar).

> Decision for the build side: Operandi's top bar already lets a user turn
> on a falloff by hand. A preset should turn its own falloff on, and turn
> it **off again when the preset is dropped**, without disturbing a falloff
> the user had on before. (Guide is silent; this is the sane reading.)

---

## 3. Tools

Status column: **Exists** = in Operandi; **Partial** = there but missing
options; **New** = not built. "Check" = I couldn't confirm from the
research side.

### Move Tools

**Element Move** (T) — *Exists (Tweak), partial.* Already specified in the
Tweak spec. Grab a vertex, edge or polygon under the pointer and drag it,
no selection needed. Options from the guide still worth checking against
Operandi: Mode (Auto picks the element type under the pointer, Manual uses
the current selection mode), Range (spherical soft area, right-drag sets
it, shown as a wire sphere), Connected Elements (Ignore, Use Connectivity,
Rigid Connections, Edge Loops), Always Raycast, Shape Preset, Slip UVs.
Gaps go to the Tweak backlog, not this panel.

**Element Push** — *New. Tier 1.* Like Element Move, but the drag moves the
element under the pointer **along its normal** instead of freely. Same
element falloff (Range, Connected Elements, Shape). Options: Distance
(starts 0), Selection Normals (polygon mode only: use the normal of the
selected polygons as a group rather than per point). Click without drag
shows a push handle on the element. Build it as Tweak with Push swapped in
for Move.

**Flex** (Shift+Y) — *New. Tier 2.* Bend-like organic deformation of the
**selection and a soft band around it**. The full move/rotate/scale
handle appears at the selection's **border** (the action center sits on
the boundary of the selection, axis from the selection). The falloff is a
**Selection falloff**: full weight inside the selection, fading over
**Steps** rings of polygons outside it (Steps starts at 2). Options:
Position, Scale, Haul (what an off-handle drag does: Translate default,
Rotate, Scale), Absolute Handle, Negative Scale, Slip UVs, Morph, plus
the falloff's Steps, Shape, In/Out, Mix. Needs: Selection falloff and the
Selection Border action center if Operandi lacks them. Guide tip: select
a closed loop and Flex to pose a limb.

**Soft Drag** — *New. Tier 2.* Move with a **Screen falloff**: a circle on
the screen around the pointer; points inside it move with the drag,
fading to zero at the circle's edge. Right-drag sets the circle size.
**Transparent** (off by default): when off only geometry facing the
view is affected; when on it reaches through the mesh. Works with or
without a selection. Needs a Screen falloff.

**Soft Move** (in Soft Drag flyout) — *New. Tier 1.* Move with a **Radial
falloff** (Linear shape). Click shows the move handle and auto-fits the
sphere. Pure preset; Operandi has both parts.

**Shear** (in Soft Drag flyout) — *New. Tier 1.* Move with a **Linear
falloff** (Auto Size on, Linear shape). Points at the Start end move fully,
points past the End not at all, so a move perpendicular to the line slants
the mesh. Pure preset.

**Smooth** — *New. Tier 1.* Evens out bumpy surfaces by pulling each
selected point toward the average of its neighbours, repeated. Works on
the selection (whole mesh if none). Options: **Strength** (amount per
pass), **Iterations** (number of passes), **Lock Boundary** (points on open
borders stay put), **Lock Corner** (border points used by only one polygon
stay put), **Preserve Volume** (keep points near the original surface so
the shape doesn't shrink), **Lock Sharp Edges** with **Sharp Angle** (edges
whose two faces meet at more than this angle stay put), Slip UVs.
Interaction: drag in the view to scale Strength interactively, or type
values. Respects an active falloff. Topology unchanged. The same tool also
works in a UV view on UVs (later). Build note: one panel tool; Relax Mesh
(below) is the one-shot command cousin.

**Jitter** (in Smooth flyout) — *New. Tier 2.* Moves each selected point
(or item) by a random offset within **Range X/Y/Z** (starts 0, 0, 0),
driven by **Seed** (starts 666; same seed = same result). Per-axis enable
switches (all on). **Rigid Translate / Rigid Rotate / Rigid Scale**: treat
each connected piece as one rigid object and jitter the piece as a whole
(used to randomise cloned fence posts). Drag in the view sets Range
interactively. Respects falloffs. Random generator: Operandi's own,
documented; it doesn't need to match the reference's numbers.

**Quantize** (in Smooth flyout) — *New. Tier 3.* Snaps the selected points
onto a 3D grid of a given step size per axis. Niche; options to be read
when it is picked.

**Soft Selection Move / Rotate / Scale / Transform** — *New. Tier 3.* Move,
rotate, scale, or all three at once, with the selection moving rigidly
and the effect fading **outside** the selection over a **Radius**
(distance from the selection, not rings). Options: the transform's fields
(Transform adds Haul, Absolute Handle, Negative Scale), Radius, **Use
Connectivity** (only connected geometry is affected), Shape, In/Out, Mix,
and **Show Vertex / Show Edge** (draw the weights on the mesh as a colour
ramp). Action center: Selection. Needs a Soft Selection falloff
(distance-from-selection). Overlaps Flex; build after Flex.

### Rotate Tools

**Twist** — *New. Tier 1.* Rotate with a **Linear falloff**. The rotation
axis comes from the Auto axis rule; the falloff line is auto-fitted on
the first click, perpendicular to the pointer. Dragging rotates: points
at the Start end turn by the full Angle, points past the End not at all,
which wrings the mesh like a towel when the axis runs along the line.
Options: Angle (starts 0°), Advanced Handles, Slip UVs, Morph, plus the
Linear falloff fields. See TW-Q1 about how a weighted rotation should
blend.

**Bend** (Ctrl+E) — *New. Tier 2.* Curls the mesh into an arc, like
bending a straight tube into part of a ring. Click at the **base** of the
bend (where the click meets the work plane); the work plane also sets the
starting orientation. Two handles appear: a **spine** (a line from the base
saying which way the bend runs and how long the bending zone is) and an
**angle disc** (drag around it to set how far it bends; dragging further
from the disc gives finer control). Points at the end of the spine get
the full Angle; the amount fades to none at the base. Options: **Angle**,
**Spine X/Y/Z** (end of the spine), **Both Sides** (also bend the part
behind the base, the other way), Slip UVs, Morph, Action Center Auto,
Action Axis Auto, **Two Handles** (extra handles to tilt the bend off the
axis). Geometry target: a straight strip along the spine becomes a circular
arc whose total turn is Angle, keeping its length (BN-Q1).

**Vortex** — *New. Tier 2.* Rotate with a **Cylindrical falloff**: full
effect at the cylinder's axis, fading to none at its radius, and endless
along the axis (Y by default). The rotation centre is the click point.
Result: a whirlpool, the middle turns most. Options: Angle, Advanced
Handles, Cylinder Centre, Size, Axis, Auto Size, Shape, In/Out, Mix.
Needs a Cylinder falloff (Radial with one axis ignored; the falloff spec's
"zero Size on one axis" rule already behaves this way, so it may be
nearly free).

**Swirl** — *New. Tier 1.* Rotate with a **Radial falloff**, centre and axis
automatic. Like Vortex but the effect also fades along the axis
(a sphere, not a cylinder). Pure preset.

### Scale Tools

**Push** — *New. Tier 1.* Moves every selected point along its own normal
by **Distance** (starts 0; negative pulls in). Interactive: drag the
push handle. Option **Selection Normals**: Averaged (normal from all
polygons around the point), Selected (only from selected polygons, so a
patch pushes out as a slab). Respects falloffs (the falloff spec already
has a Push oracle: each point moves Distance × weight). Topology unchanged.

**Sculpt** — *New. Defer to the Sculpt milestone.* The old single-brush
sculpt tool: drag a brush circle across the surface to push it in or
out. Options include Offset Mode (Adaptive or Absolute), Offset Amount,
Smooth Amount (Shift smooths while brushing), Normal Mode (Brush Center,
Vertex, Average, Screen, Mouse Down, X, Y, Z), Density Mode, brush Size
in pixels (right-drag), brush Shape preset. It overlaps the planned
Sculpt work, so build it there, not here.

**Taper** — *New. Tier 1.* Scale with a **Linear falloff** (Auto Size on).
Off-handle drags scale (Haul Scale). Points at the Start end scale fully,
past the End not at all: a column becomes a cone or flares. Options:
Scale X/Y/Z, Absolute Handle, Negative Scale, the Linear falloff fields.

**Bulge** — *New. Tier 1.* Scale with a **Radial falloff** using the
**Smooth** shape. Click places the falloff centre where the pointer meets
the work plane, sized to the selection; dragging the stretch handles
swells or pinches the middle while the edges of the sphere stay put.
Right-drag redefines the sphere. Options: Scale X/Y/Z, Absolute Handle,
Negative Scale, Radial falloff fields.

**Flare** (in Bulge flyout) — *New. Tier 2.* Push (along normals) with a
**Linear falloff**: one end of the mesh swells outward, fading to nothing
at the other. Options: Distance and the Linear falloff fields. Pure
preset once Push exists.

### Align Tools

**Radial** (Radial Align) — *New. Tier 2.* Moves the selected points onto
a **circle** (or a regular polygon). Edge mode: selected edges are joined
into a loop and its points are placed on the circle. Vertex mode: points
follow their edge connections; if they aren't connected, selection order
is used. Polygon mode: the selection's border is used, and inner points
are relaxed. The circle's centre and radius are measured from the
selection when the tool starts; handles change centre, radius and the
start angle. Options: **Mode** (Circle, N-Sided), **Side** and **Rotate**
(for N-Sided; if fewer points than sides, extra points go between the
corners), **Radius**, **Start Angle**, **Central Angle** (less than 360°
makes an arc), **Weight** (0..100%: blend between original and aligned
position; drag sets it; falloff can scale it), **Smoothing Iterations**
(relax inner points, polygon mode), **Flatten** (put inner points on the
circle's plane first), **Exponent** (positive squares the circle off
toward a rounded square, negative pinches it concave), **Center X/Y/Z**.

**Linear** (Linear Align) — *New. Tier 2.* Lines the selected points up
between the two **end points** of each selected run. Edge mode: connected
edges form runs (runs must not cross); vertex mode as Radial; polygon mode
uses the border. Options: **Mode** (Line: straight; Curve: fit a smooth
curve), **Uniform** (even spacing; off = each point moves to its nearest
spot on the line), **Preserve Original Curve** (Curve mode), **Weight**,
**Smoothing Iterations**, **Flatten**, **Align Length** with **Edge
Length** (open edge runs only: resize every run to a length; the last
selected run sets the starting length), **Align to First Selected**,
**Screen Align** (straighten as seen on screen, keep depth; stores the
view direction in **Axis Vector X/Y/Z**).

**Quick Align, Ground Align** — *Exists* (Create panel). Same buttons here.

### Commands

**Relax Mesh...** — *New. Tier 2.* One-shot command with a dialog. Evens
out a detailed mesh into a smooth, sculpt-ready surface by repeated
relaxing. Options: **Iterations** (starts 5) and **Constrain** (keep
extra constraints such as borders; exact meaning RM-Q1). One undo step.
Build it on the same relax routine as Smooth.

---

## 4. What Operandi needs underneath

| Piece | Used by | Probably in Operandi? |
|---|---|---|
| Move / Rotate / Scale / Transform with weights | all presets | Yes |
| Linear falloff | Shear, Twist, Taper, Flare | Yes (falloff spec) |
| Radial falloff | Soft Move, Swirl, Bulge | Yes (falloff spec) |
| Push transform (move along normals) | Push, Flare, Element Push | New |
| Element falloff | Element Move, Element Push | Yes (Tweak) |
| Cylinder falloff | Vortex | New (small) |
| Screen falloff | Soft Drag | New |
| Selection falloff (rings) + Selection Border action center | Flex | Check |
| Soft Selection falloff (distance) + weight display | Soft Selection family | New |
| Relax / smooth routine | Smooth, Relax Mesh, Align smoothing | New |

---

## 5. Open questions (for a bridge pass later)

- **TW-Q1** Under a falloff, the falloff spec found that Rotate **blends
  positions** (p + w(R(p) − p)), not angles. If Twist does the same, a big
  twist pinches the middle. Does Twist (and Vortex, Swirl) rotate each
  point by w × Angle instead? Sane default if unanswered: rotate by
  w × Angle, since that's what a twist looks like.
- **BN-Q1** Bend's exact arc: does the bent part keep its length along the
  spine, and what happens to points beyond the spine's end (rigidly
  carried along, presumably)?
- **SM-Q1** Smooth's per-pass rule and how Strength scales it; what
  Preserve Volume does exactly.
- **RM-Q1** Relax Mesh's Constrain option.
- **SD-Q1** Soft Drag: does it need a selection, or does it act on
  whatever is under the circle?
- **DF-Q1** Starting values not in the config: Smooth Strength and
  Iterations, Sharp Angle, Flex Steps confirmed 2, Radial/Linear Align
  defaults, Bend Angle. A read-only scripted dump would fill these in one
  pass.
- **DF-Q2** Do the presets leave their falloff on after drop? (Assumed
  no, see §2.)
