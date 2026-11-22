# UV Unwrap, Pack UVs, UV Relax and the UV viewport — behavior spec v1.1

Provenance: user guide sections "UV Unwrap" (incl. Creating Seams, Unwrap &
Relax), "Pack UVs", "UV Relax", "UV Viewport" and "Working with UV Maps" for
names, options and intent; field labels, option order, starting values and
greyed-field notes as Modo 17.1v1 reports them in its own UI text and its
tool list; UV viewport starting display settings from the reference's
plain-text layout files; scripted probes in Modo 17.1v1 (2026-11-11) on a
1 m cube for selection, seams, map creation, layout, Pack and Relax.
REA-assisted: no.

Scope: the interactive tools and the one-shot Pack command as a user meets
them. Procedural (mesh operation) versions are listed only where they add a
field. Marked *(build guess, UV-Qn)* means the docs don't settle it; pick
the stated behavior and keep it easy to change.

The unwrap and pack engines themselves are Operandi's own. This spec fixes
what the user sees: fields, defaults, what's selected, what gets written,
how the result is laid out.

---

## 1. UV Unwrap tool

### Summary

Flattens the visible surface of the active mesh into the current UV map,
cutting it apart along seam edges into islands and laying the islands out
side by side. It's an interactive tool: the result updates live as fields
change, and the tool stays active until dropped.

### Inputs

- Works on every **visible foreground polygon** of the active mesh layers.
  Hidden polygons are left alone (observed: a hidden face kept its UVs),
  so hiding is how a user limits the tool. A **polygon selection does not
  limit it** (observed: with one face selected, all six faces were
  re-unwrapped).
- **Seams** come from one of three sources (the UV Seam Type field):
  - **Selected Edges** (the default): the current edge selection. With no
    edges selected on a closed mesh, **every polygon becomes its own
    island** (observed on the cube: 6 islands). On an open mesh with no
    seams, expect cuts only at its borders *(build guess, UV-Q2)*.
  - **UV Seam Map**: edges stored in a named seam map (see Cut Map, §1.6).
  - **Selection Set**: edges in a named edge selection set.
- Needs no particular selection mode. Edge mode is the usual one because of
  the default seam source.

### Options (Tool Properties, in this order)

| Field | Type | Default | Range / choices | Effect |
|---|---|---|---|---|
| Iterations | integer | 100 | 0 and up | How far the solver runs. 0 shows only the Initial Projection (no unwrapping). More gives a better result up to a point. |
| Relative Gap Size | percent | stored 0.2 | 0 and up | Space left between islands and at the tile edge. Larger values push islands apart and shrink the whole layout to make room. Observed: 0 gives a layout touching the tile edge; the default leaves about 0.005 at the edges of a cube layout. How the UI shows 0.2 (0.2% or 20%) is UV-Q16. |
| Segment by Material | boolean | off | | Cuts between polygons with different material tags, so each material becomes its own island(s). |
| Segment by Part | boolean | off | | Same, using part tags. |
| Initial Projection | choice | Planar | Planar, Cylindrical, Spherical, Use Current Map, Group Normal | Starting layout before solving. Use Current Map starts from the UVs already in the map (refinement without losing them). Group Normal projects along the average normal of the selected polygons. |
| Pinning Axis | choice | U | U, V | The UV axis favored when the solver shuffles island borders. |
| Projection Axis | choice | Y | X, Y, Z | Axis for Planar, Cylindrical and Spherical. Greyed for the other two projections, with a note saying it applies only to Planar, Cylindrical or Spherical. |
| Offset Angle | angle | 0° | any | Rotates the unwrapped result. Live. |
| Seal Holes | boolean | on | | Treats holes in the surface (an open eye socket) as if capped while solving, so they keep their shape. No polygons are added to the mesh. |
| Update Layout | boolean | on | | Keeps the layout inside the 0–1 tile. Observed: off gave exactly the same result on the cube, so its effect is still UV-Q3. Build it as a pass-through flag for now. |
| Method | choice | Conformal | Conformal, Angle Based | Two solver styles. See note below. |
| Symmetrize | boolean | off | | On a symmetric mesh, produces a mirror-symmetric layout. May need more Iterations. |
| Symmetry Axis | choice | X | X, Y, Z | Axis the mesh is symmetric across. |
| Mirror Axis | choice | U | U, V | UV axis the result mirrors across (usually the same as Pinning Axis). |
| UV Seam Type | choice | Selected Edges | Selected Edges, UV Seam Map, Selection Set | Seam source (see Inputs). |
| UV Seam | map name | (empty) | seam maps of the mesh | Shown when the type is UV Seam Map. |
| Selection Set | name | (empty) | edge selection sets | Shown when the type is Selection Set. |

Symmetry Axis and Mirror Axis only matter when Symmetrize is on *(build
guess: grey them otherwise)*. The procedural version adds a **UV Map** name
field (default "Texture").

**Method note.** The reference's own option help ties **Conformal** to
least-squares conformal mapping and **Angle Based** to angle-based
flattening. The user guide states the pairing the other way round. Build it
to match the option help, which also matches the labels' plain meaning
(Conformal = least-squares conformal, Angle Based = angle-based
flattening) *(UV-Q4 asks which the reference actually runs)*. The guide's
advice still holds as a hint: one suits organic shapes, the other hard
surfaces, and users try both.

### Interaction

- **Activation**: the Unwrap button in the UV tools (UV layout left panel,
  UV menu of the Modo/Model layouts), or the **Unwrap Tool** button in the
  Cut Map popover, which also sets UV Seam Type to UV Seam Map and picks the
  current seam map.
- **First result**: the unwrap appears as soon as the tool is active and
  updates live when any field changes, for meshes small enough to be
  real-time *(build guess, UV-Q5: whether it applies on activation or only
  on the first click)*.
- **Drag**: click and drag in the UV viewport (or 3D viewport) to raise or
  lower Iterations. Horizontal drag, right = more *(build guess, UV-Q6)*.
- A status line reads "Unwrapping..." while a long solve runs.
- **Drop**: the result stays when the tool is dropped (another tool, Space,
  Esc follow the usual tool rules in Operandi).

### Result

- The selected UV map is overwritten for the polygons the tool used. Other
  polygons' UVs are left as they were.
- Each seam edge becomes a break: the vertices along it get separate UV
  positions on each side (discontinuous UVs). Islands are the connected
  groups that remain.
- Layout: all islands are scaled together (aspect kept) so the layout's
  longer side spans the tile, anchored at the lower-left corner (0, 0).
  Observed with gap 0: a cube cut at its top rim and four verticals gives
  a cross of five faces plus the cap, spanning exactly 1.0 × 0.75.
  Relative Gap Size then insets and separates the islands.
- Island orientation follows Pinning Axis; Offset Angle then rotates it.

### Which UV map is written

- The UV map currently selected for the mesh (Lists → UV Maps, the UV
  viewport's map button, or the left panel's Map field). If several are
  selected, the first one *(build guess, UV-Q7; the guide states this for
  Pack)*.
- If the mesh has **no UV map**, a new one named **Texture** is created
  (observed). One observed run with no map and no seams created the map but
  left every UV at (0, 0). Operandi should give the normal result instead.

### Selection after

Selection is unchanged.

### Refusals

- No visible polygons: nothing happens, no message *(build guess)*.
- Seam source is a map or set that doesn't exist or is empty: unwrap with
  no seams *(build guess, UV-Q2)*.

### Undo

One undo step per tool session: everything from activation to drop undoes
together, restoring the UV map as it was *(build guess, matches Operandi's
existing tool rule)*.

### 1.5 Unwrap & Relax tool

One tool that runs Unwrap and then Relax. Fields are the Unwrap fields plus
the UV Relax fields (§3), plus an on/off for each half.

Its own starting values where they differ from the separate tools:
Iterations 0 (unwrap half), relax Mode **Angle Based**, relax Iterations 0,
Ignore Edge Selection **on**. Seal Holes and Update Layout on.

Activation: UV menu → UV Unwrap & Relax, or Cut Map → Unwrap & Relax (which
sets UV Seam Type to UV Seam Map). A **single click** in the 3D or UV
viewport applies it. It has no Interactive (pin) mode; use UV Relax for that.

### 1.6 Cut Map popover (seam maps)

A popover in the UV tab that builds seams up in stages:

- **UV Seam**: picks the seam map; "(new)" opens the new vertex map dialog
  (name, type).
- **Add Seam**: adds the selected edges to the seam map. With **no edges
  selected, every edge** is added.
- **Clear Seam**, **Delete Seam Map**.
- **Show UV Seam** on/off, and **Seam Color**. Starting seam colour is a
  pale lavender, RGB (0.6, 0.6, 1.0).
- **Unwrap Tool** and **Unwrap & Relax** buttons start those tools with the
  seam map as the source.

---

## 2. Pack UVs

### Summary

Rearranges the islands of the current UV map so they fill a target region
without overlapping: moves them, optionally scales and rotates them, and
leaves a gap between them. It's a one-shot command with an options dialog
(Pack UVs... → dialog → OK), not a live tool.

### Inputs

- Acts on the **selected UV map** of the active mesh(es). With several maps
  selected, only the first is packed.
- Which islands: with polygons selected, **only the islands containing them
  move**. The others stay exactly where they are, and the moved islands keep
  clear of them (observed). With nothing selected, all islands are packed.
- With several meshes active, their islands pack together into one layout
  (By Item region separates them instead).

### Options (dialog, in this order)

| Field | Type | Starting value | Effect |
|---|---|---|---|
| Pack | boolean | on | Islands may move. Off keeps them in place (only Stretch/Orient apply). |
| Stretch | boolean | on | Before packing, rescales and reshapes each island so its UV area matches its 3D area (evens out density). |
| Orient | boolean | off | Islands may rotate while packing. |
| Keep Size | boolean | off | Islands keep their size (no scaling to fit). Disables Stretch. |
| Direction | choice | Auto | Horizontal, Vertical, Auto (that order in the list): preferred proportion of each island (wide, tall, or whichever fits). |
| Gaps | percent | stored 0.2 (display: UV-Q16) | Space between islands, as a fraction of the region. |
| Set Gaps by Pixel | boolean | off | Use Pixel Size instead of Gaps. |
| Pixel Size | number | 10.24 | Gap in pixels. Gaps = Pixel Size / image size, where the image size is the Texel Density panel's Image Size (default image size to confirm, UV-Q10). |
| Based on Bounding Box | boolean | off | Treat each island as its bounding rectangle (faster, looser). |
| Lock Stacked | boolean | off | Islands lying exactly on top of each other (same UV positions, e.g. mirrored halves or copies) move as one stack. |
| Pack Region | choice | Normalized | Normalized, Nearest, UDIM, Manual, Tiles, Polygon Tag, By Item (see below). |
| UDIM | integer | 1001 | Target tile when Pack Region is UDIM. |
| Region U, Region V | UV | 0, 0 | Manual region's lower-left corner. |
| Width, Height | UV | 1, 1 | Manual region's size. |
| Tile U, Tile V | integer | 1, 1 | Tiles region: number of whole tiles wide and high. |
| Polygon Tag | choice | Material | Material, Part, Smoothing Group, Selection Set. Used by the Polygon Tag region. |
| Background Constraint | boolean | off | Packs into the empty space left by islands that aren't being packed. |
| Pack UVs to New Map | boolean | off | Writes to a new map and keeps the original. With Polygon Tag or By Item, one new map per tag or item, named after it. |
| New Texture | name | *(UV-Q10)* | Name for the new map. |
| Find Best Box Rotation | boolean | on | Listed by the reference; not described in the guide (UV-Q11). |

Starting values above were read from Modo 17.1v1. The dialog keeps its last
values between uses, so these may be remembered values rather than factory
ones (UV-Q10). Note that Orient starts **off**, so by default islands are
not rotated, despite the guide's summary. The
procedural version adds Enable, Use World Transform, a UV map Name, and
Texel Density Image Size (the direct command takes that from the Texel
Density panel).

**Pack Region choices**

- **Normalized**: fill the 0–1 tile. This is the "fit to 0..1" behavior.
- **Nearest**: fill the whole tile nearest each island's current place.
- **UDIM**: fill the chosen UDIM tile.
- **Manual**: fill the rectangle Region U/V + Width/Height. Example: corner
  (-1, -1), size (2, 2) treats -1..1 as one area.
- **Tiles**: fill a block of Tile U × Tile V whole tiles; islands don't cross
  tile borders.
- **Polygon Tag**: group islands by the chosen tag; each group goes to its
  own UDIM tile (or its own new map with Pack UVs to New Map). One island
  that carries two tags is split so each tag's polygons go to their group.
- **By Item**: each mesh item goes to its own UDIM tile (or its own new map,
  named after the item).

### Result

- With the starting values: every island is moved and scaled so the set
  fills the region. All islands get **the same scale** (observed), with no
  island overlapping another. Packing follows island shapes, not boxes:
  the cube's cap island was tucked into the empty corner of the cross
  island. Observed on the 1 × 0.75 cross-plus-cap layout: each face becomes
  0.3254 wide, with a 0.01 margin at the tile edge and about 0.004 between
  islands.
- Packing selected islands only: they may be rescaled to fill the free
  space (observed: the lone cap grew from 0.3254 to 0.3356).
- Overlapping islands (overlap shown red) are separated, unless they're an
  exact stack and Lock Stacked is on.
- Island shapes are not changed, except by Stretch.

### Refusals

- No UV map selected: does nothing; tell the user to select a UV map
  *(own wording)*.
- Status line "Packing..." while it runs.

### Undo

One undo step.

---

## 3. UV Relax tool

### Summary

Smooths an existing UV map to reduce stretching, like a smoothing brush for
UVs. Interactive tool with live results.

### Inputs

- The selected UV map, on visible polygons. A **polygon selection does not
  limit it** (observed: with one face selected, faces in both islands
  moved).
- **Ignore Edge Selection** (default on): selected edges are ignored. When
  **off**, every island that contains a selected edge is **held still** and
  only the other islands relax (observed with one edge selected).

### Options (Tool Properties, in this order)

| Field | Type | Default | Choices | Effect |
|---|---|---|---|---|
| Mode | choice | Conformal | Spring, Conformal, Angle Based, Adaptive | Spring treats edges as springs (poor with overlaps). Conformal and Angle Based use the same two styles as Unwrap. Adaptive tries to keep each polygon's relative size from 3D. |
| Iterations | integer | 0 | 0 and up | Amount of relaxing. Dragging in the viewport sets it. |
| Interactive | boolean | off | | Pin mode (below). |
| Seal Holes | boolean | off | | As in Unwrap. Active only in Conformal and Angle Based (greyed otherwise, with a note). |
| Boundary | choice | Smooth | Smooth, Locked, Maintain, Straighten | Island borders: Smooth relaxes borders and inside together. Locked holds borders fixed. Maintain lets border points slide only along the border (keeps symmetry). Straighten makes border runs straight, then relaxes inside. Greyed in Spring mode *(guess at which field the "not Spring" note belongs to)*. |
| Uniform Spans | boolean | off | | Evenly spaced border points. Only with Boundary = Straighten. |
| Pause | boolean | off | | Stops live re-solving so pins can be placed. Only with Interactive. |
| Area Weight | percent | 10% | | How strongly polygons resist changing area. Only in Adaptive. |
| Save Constraints | boolean | off | | Remember the pins for the next use of the tool. Only with Interactive. |
| Ignore Edge Selection | boolean | on | | See Inputs. |
| UV Symmetry | choice | Off | Off, U, V | Keeps the relaxed result symmetric across U or V. |
| Show Transform Handles | boolean | off | | Shows a move/scale/rotate handle for the relaxed area: Offset U/V 0, Scale U/V 100%, Pivot (0.5, 0.5), Angle 0° *(UV-Q13: exact use)*. |

The procedural version adds a **UV Map** name field.

### Interaction

- Activate from the UV tools (Relax). Click and drag in the UV viewport to
  raise or lower Iterations; the map updates live.
- **Pins (Interactive on)**: clicking a UV vertex adds a pin, drawn as a
  small blue square. Ctrl/Cmd+click on a pin removes it. With two or more
  pins and enough Iterations, dragging a pin drags the map with it, stretched
  between the pins; the other pins stay put. A handful of pins gives the
  cleanest result. Pause holds the
  solve while pinning. Save Constraints keeps the pin set for next time.

### Result / Undo

UVs of the affected polygons change; topology and 3D positions never do.
One undo step per tool session *(build guess)*.

---

## 4. UV viewport

### Layout and map choice

- A 2D view of UV space: U to the right, V up, the 0–1 tile being the
  texture's home. Outside 0–1 the texture repeats.
- **Map button** at the top ("UV: (none)" or the map's name): picks the UV
  map shown and edited, or creates a new one. The same choice is in Lists →
  UV Maps and in the UV layout's left-panel Map field; all three stay in
  sync.
- When several mesh items share a map name, the active item's UVs draw white
  and the others (background items) draw dark. **Show Inactive UVs** shows or
  hides those background items' UVs. Painting or lasso-selecting in Items
  mode in the UV view picks which item is active.

### Starting display (UV layout's UV viewport)

| Setting | Start |
|---|---|
| Grid | on, lines every 0.1 in U and V |
| Subgrid | off, 5 divisions per grid cell when on |
| Grid extent (spans) | 0–1 (one tile); the general UV view preset uses -1..1 |
| Labels | inside on, outside off, every 0.5 |
| Axis | on |
| Geometry | on |
| Vertex indices | off |
| UDIM number | on (the tile's address at its centre) |
| Backdrop image | on, filling 0–1, aspect off, brightness/contrast/transparency 0 |
| Normal-space tint | 0 (the general UV view preset uses 5%) |
| View centre | (0.5, 0.5) |
| Zoom | 300 screen pixels per UV unit *(guess at the unit, UV-Q14)* |

Options are in a properties panel (gear icon, or O over the viewport) and
the most used ones are toolbar toggles: Auto Spans, Show Overlap, Show
Distortion, Show Image, Show Inactive UVs, Show Wireframe.

Other display fields: U/V spans (how many tiles the grid draws), Auto Spans
(grid fills the whole view), image extents, Show Inside/Outside Label, Label
Space, Fill Polygons (faint face fill), Show Selections and Fill Selected
(orange), Show Selection Rollovers, Use Image Aspect (stretch the view to the
backdrop image's aspect), image Invert/Contrast/Brightness/Transparency.

### Overlays

- **Overlap**: areas where one UV polygon covers another are tinted red.
- **Flipped** (Options → Show Flipped): back-facing islands tinted yellow;
  where both apply the colours blend.
- **Distortion** (Show Distortion): each polygon is coloured by its UV size
  relative to its 3D size, compared with the rest of the map. Neutral is a
  mid gray-green; relatively too small runs toward red; relatively too large
  toward blue. The same colouring can be shown in the 3D view.
- **Discontinuous edges** (where one 3D vertex has two UV positions, i.e.
  seams already cut in the map): pale lavender, RGB (0.6, 0.6, 1.0).
- **UV island borders in 3D** (Show UV Boundaries option of the 3D view):
  purple, RGB (0.65, 0.15, 0.93).
- **Coverage**: a percentage in the lower-right corner: how much of the
  tile area is covered by UVs. With some items selected it shows the
  selected items' share and the rest separately. Updates while dragging and
  on release.
- **Checker**: the reference has no built-in checker overlay. Users load a
  UV check image as the backdrop or as a texture. *(Operandi may add a
  checker toggle; that's a new feature, not a match. UV-Q15.)*

### Selection in UV space

- The usual Vertices / Edges / Polygons modes work in the UV view and share
  the selection with the 3D view *(sharing: build guess)*.
- Click, paint (drag) and lasso (right-drag) select as in 3D.
- **Select Connected** in the UV view selects the **UV island** (connected in
  UV), not the whole 3D-connected piece.
- Rollover highlight follows the cursor when Show Selection Rollovers is on.
- Hide (H, Shift+H, Ctrl/Cmd+H, U) and Lock (J, Shift+J, Ctrl+J, I) work as
  in 3D; hidden polygons drop out of UV tools.

### Navigation

| Input | Action |
|---|---|
| Alt + drag | pan |
| Ctrl/Cmd + Alt + drag | zoom about the cursor |
| Mouse wheel | zoom about the cursor |
| Shift + Alt + middle-drag | pan in U only |
| Shift + Alt + right-drag | pan in V only |
| Ctrl/Cmd + Alt + right-drag | box zoom |
| A | fit all UVs in view |
| Shift + A | fit the selection |
| . and , | zoom in / out |
| G | centre on the cursor |

There is no rotation: the view stays flat.

---

## 5. Names

UI names to use: **UV Unwrap** (tool), **UV Unwrap & Relax** (tool),
**UV Relax** (tool), **Pack UVs** (command with dialog, menu label
"Pack..."), **Cut Map** (popover). Pack's command arguments follow the
dialog's field order above. Exact script names and argument names go
through Operandi's command registry, not this spec.

## Acceptance cases

See `cases.json`. UVW-10..13, UVP-08..09 and UVR-06..07 are observed in
Modo; the rest are behavior checks from the guide and UI facts. UVW-01..07 Unwrap, UVW-08..09 Unwrap &
Relax / Cut Map, UVP-01..07 Pack, UVR-01..05 Relax, UVV-01..06 viewport.

## Open questions

- ~~UV-Q1~~ Answered: a polygon selection doesn't limit Unwrap.
- **UV-Q2** Partly answered: a closed mesh with no seams gives one island
  per face. Still open: an open mesh with no seams, and an empty seam map.
- **UV-Q3** Update Layout off made no difference on the cube. What does it
  change?
- **UV-Q4** Which solver does each Method label actually run? (Guide and
  option help disagree.)
- **UV-Q5** Does Unwrap apply on activation, or on the first click?
- **UV-Q6** Drag direction and rate for Iterations.
- **UV-Q7** Unwrap with several UV maps selected.
- ~~UV-Q8~~ Answered: a map named Texture is created.
- ~~UV-Q9~~ Answered: only islands with selected polygons move; the rest
  stay and are avoided.
- **UV-Q10** Mostly answered. Are the Pack starting values factory values
  or remembered ones? New Texture's default name and the Texel Density
  Image Size default are still open.
- **UV-Q11** What Find Best Box Rotation does.
- ~~UV-Q12~~ Answered: islands holding selected edges stay still.
- **UV-Q13** Relax's transform handles: what they move.
- **UV-Q14** Units of the viewport zoom value.
- **UV-Q15** Checker: confirm there is no built-in checker overlay.
- **UV-Q16** How the UI shows a stored gap value of 0.2 (as 0.2% or 20%),
  and what the gap is relative to.
- Out of scope for v1: Peeler, Create UV (projection tool), Fit UVs, Orient
  Pieces, Align, Sew, Split, Real Size, UV export, procedural mesh-op stacks.

Still open after the 2026-11-11 probes: Q2 (open meshes), Q3, Q4, Q5, Q6,
Q7, Q10 (factory or remembered), Q11, Q13, Q14, Q15 and Q16. Q5, Q6 and the
display questions need a person at the mouse. Undo granularity can't be
checked by script, so it stays a build guess.
