# UV tools: behaviour spec v1 (the rest of the UV tab)

Rules are **UVT-nn**, open questions **UVT-Qnn**. Provenance: the
reference's public user guide, UV chapter (paraphrased); its plain-text
toolbox and preset settings for button names, order, sub-options and
which buttons are presets. Not measured through the bridge, so every
starting value not stated by those sources is a marked **build guess**.
REA-assisted: no.

Already specified elsewhere (don't repeat, build from those):
- `uv-unwrap-pack-relax-2026-11-11` v1.1 (**UVS-1**): Unwrap, Unwrap &
  Relax, Cut Map basics, Pack, Relax, the UV viewport.
- `tutorial-gaps-2026-11-13/SPEC-UV-EDITING.md` v1 (**UVE-1**): UV map
  list, Move/Rotate/Scale in the UV view, Fit, Project from View, Mirror,
  Align, map copy and paste.

This spec fills in everything else, and adds detail to UVE-1 where the
guide gives more. The engine names (E1..E15) are from
`UV-TOOL-FAMILIES.md`.

---

## 0. Getting to UV editing

**UVT-200 Three ways in.** A UV layout tab; a UV Editor window (Texture >
Open UV Editor, a floating UV view about 800 x 600 with the common UV tools
down its right side, usable while staying in Model or any layout;
double-clicking a UV map in the UV map list also opens it); later, palette
buttons. Any viewport can still switch to a UV view through its
viewport-type control. Double-clicking the UV layout tab resets that layout
to its defaults.

**UVT-201 UV layout, three columns.** Left: the tabbed toolbox open at the
UV tab, upper part of the column, active tool properties below it. Centre:
UV view on the left half and a Perspective 3D view on the right half,
50/50; selection shared. Right: the usual tabs (Items, Properties, Lists
with the UV map list, Shading, etc). UV view header holds the UV: (map)
button and display toggles. The toolbox's Map field at the top of the UV
tab is the same choice as the header's.

**UVT-202 Keys.** Keys in a UV view as already built (W/E/R/Y start
transforms on UVs; no single key opens UV editing).

**UVT-203 Build order.** 1) UV layout tab as UVT-201; 2) Texture > Open UV
Editor as a floating window (floating panels are their own OS window, per
Jason's rule / BIBLE UIC-183), and double-click on a map in the UV Maps list
opens it; 3) keep View > UV View as the per-view switch. Palette buttons and
pie-menu entry: later, not now.

---

## 1. Rules shared by every UV tool

**UVT-01 Current map.** Every UV tool reads and writes the mesh's current
UV map (UVE-1 §1). A tool that writes UVs on a mesh with no UV map first
makes one named "Texture" (UVS-1 observed this for Unwrap; the guide says
the same for Project from View).

**UVT-02 Scope.** Unless a tool says otherwise: with polygons, edges or
points selected, the tool acts on the UVs of that selection; with nothing
selected it acts on all visible polygons of the active meshes. Hidden
polygons are never touched. (Unwrap and Relax are exceptions, see UVS-1:
a polygon selection doesn't limit them.)

**UVT-03 UV points.** A mesh point can have several UV positions, one per
island it borders (a "discontinuous" point). Selecting in a UV view
selects the UV positions under the cursor. A 3D point selection selects
all of its UV positions.

**UVT-04 Island.** A UV island is a group of polygons joined through shared
UV positions (not just shared 3D points). Every "per island" rule uses
this.

**UVT-05 Gaps.** Several tools have **Gaps** (a percentage of the target
region) and **Gaps by Pixel** (pixels). They're linked: Gaps = Pixel Size
÷ Texel Density Image Size. Editing one updates the other. The Image Size
comes from the Texel Density panel (§7.9).

**UVT-06 3D never moves.** UV tools change UV values only. Point positions,
polygons and other maps are untouched, except Convert UVs to Mesh (§15).

**UVT-07 Undo.** One-shot commands are one undo step. Interactive tools
are one step per tool session, as for Operandi's other tools.

**UVT-08 Remembered options.** Commands with a dialog remember the last
values used. A plain click on the button runs with the remembered values.
Ctrl/Cmd+click opens the dialog first (the guide says this for Fit;
**build guess** for the others).

---

## 2. Project (E2): Create UV tool

### 2.1 Summary

An interactive tool that writes UVs by projecting polygons from 3D onto a
flat shape: a plane, a cylinder, a sphere, six cube faces, each polygon on
its own, or automatic charts. It is usually the first step, and the result
is often tidied up afterwards.

### 2.2 Scope

**UVT-10** Selected polygons, or all visible foreground polygons when none
are selected. Only those polygons get new UVs. Other polygons keep theirs.

### 2.3 Options (in this order)

| Field | Type | Start | Effect |
|---|---|---|---|
| UV Map | map choice | current map | The map written to. |
| Create New Map | button | | Makes the map named in UV Map if the mesh doesn't have it. |
| Projection Type | choice | Planar *(build guess)* | Planar, Cylindrical, Spherical, Atlas, Barycentric, Atlas 2, Cubic (this is the guide's order). |
| Axis | X, Y, Z | Y *(build guess)* | Main direction for Planar, Cylindrical, Spherical, Cubic. |
| Settings | Automatic, Manual | Automatic | Automatic sizes the projection to the selection's bounding box so it fills the tile. Manual uses Size, Center and Rotation. |
| Size X, Y, Z / Center X, Y, Z / Rotation | numbers | from the selection | The projection shape's frame in Manual. Also drawn as a handle in the 3D view *(build guess, UVT-Q01)*. |
| Seam Angle | angle | 0° | For Cylindrical and Spherical: where the wrap-around seam falls, turned about the Axis. |
| Relative Gap Size | percent | as Unwrap | Atlas types: space between the charts. |
| Segment by Material | on/off | off | Atlas types: no chart holds two material tags. |
| Segment by Part | on/off | off | Atlas types: no chart holds two part tags. |
| Orientation | on/off | off | Atlas types: flips charts by their facing axis so left and right sides read as mirror images. Always on (locked) for Atlas 2. |
| Segments | integer | *(UVT-Q02)* | Atlas 2: roughly how many charts. |
| Iterations | integer | *(UVT-Q02)* | Atlas 2: how strictly charts group by facing. More gives less stretch and more charts. |

Fields that don't apply to the current Projection Type are greyed.

### 2.4 Projection types

**UVT-11 Planar.** Each point's UV is its position seen straight down the
Axis, like an orthographic Top, Front or Side view. Polygons on the back
land on top of the front ones (overlap is expected).

**UVT-12 Cylindrical.** U runs around the Axis (0 to 1 once round), V runs
along it. One seam where U wraps, placed by Seam Angle. Polygons that
straddle the seam are cut so they don't stretch across the whole tile.

**UVT-13 Spherical.** U around the Axis, V from pole to pole (poles on the
Axis). Same seam rule as Cylindrical.

**UVT-14 Cubic.** Each polygon is given to the one of six directions its
normal faces most (top, bottom, left, right, front, back) and projected
flat along it. Unlike Planar, nothing is mirrored back to front.

**UVT-15 Barycentric.** Each polygon fills the whole 0–1 tile by itself.
Quads hit the four corners exactly. Triangles and n-gons are placed inside
the tile as well as possible. All polygons overlap. Used to tile one image
on every polygon.

**UVT-16 Atlas.** Splits the surface into many charts automatically, each
flattened with little stretch, keeping their relative 3D sizes, packed in
the tile. Very broken up: meant for baking, not painting.

**UVT-17 Atlas 2.** Also automatic charts, but fewer and better shaped,
grouped by similar facing (Segments, Iterations), packed to fill the tile
efficiently without changing relative sizes.

Engine note (Jason 11-16): Atlas and Atlas 2 chart splitting comes from a
port of xatlas's charting (MIT). Flattening and packing reuse the Blender
port already in `operandi-uv`. See `ENGINES.md`.

### 2.5 Interaction

**UVT-18** Activating the tool projects straight away with the current
values and shows the result live. Changing a field updates it. Dropping the
tool keeps it.

### 2.6 Project from View

Extends UVE-1 §4.

**UVT-19** A separate tool (the guide calls it View Projection, the button
says Project from View). A click in a 3D view writes the selected polygons'
screen positions as UVs, as if photographing them. Move the view and click
again to redo it. **Apply** does the same as the click, but only when the
tool isn't in interactive mode. **Auto Scale** (on, build guess) fits the
result in the 0–1 tile. **Hidden Face** (off, build guess) drops polygons
that face away from the view.

---

## 3. Unwrap & Relax as a family

**UVT-20** Build Unwrap & Relax as the Unwrap engine then the Relax engine
in one tool session, not a third solver. Fields as UVS-1 §1.5. The Cut Map
buttons are the same two tools with Seam Source set to the current seam
map (the reference's own presets do exactly this).

---

## 4. Relax and the pipe

Adds to UVS-1 §3.

**UVT-21** Relax honours the falloff layer: a point's relax amount is
multiplied by its falloff weight (the guide suggests using a falloff to
limit Relax). Falloff shapes are measured in UV space in a UV view and in
3D in a 3D view *(build guess, UVT-Q03)*.

**UVT-22** Relax's UV Symmetry field (Off, U, V) is the same thing as the
UV Symmetry layer (§10). The tool field wins while the tool is active
*(build guess)*.

---

## 5. Grid layout (E5)

### 5.1 Peeler

**UVT-30** A tool for clean quad meshes such as belts, straps, arms and
pipes. The selected edges are the cut. The tool spreads UVs out at right
angles to the cut, loop by loop, until it meets the cut again from the
other side, giving a regular grid strip.

| Field | Start | Effect |
|---|---|---|
| Min U, Max U, Min V, Max V | 0, 1, 0, 1 | The UV box the strip is fitted into. |
| Uniformity | *(UVT-Q04)* | 0%: each UV polygon's size follows its 3D size. 100%: all UV polygons the same size (a perfectly even grid). |

**UVT-31** On meshes that aren't tidy quad loops, the Peeler gives poor
results. That's expected: the guide points users to Unwrap instead. Show
nothing special *(build guess)*.

### 5.2 Rectangle

**UVT-32** One-shot. Straightens each selected island made only of quads,
with the same number of spans along each pair of opposite sides, into a
rectangle with a regular grid inside.

| Field | Start | Effect |
|---|---|---|
| Pack | off *(build guess)* | Packs the resulting rectangles in the tile without overlap, scaling them, with Gaps between. |
| Gaps | as Pack | Space between islands when Pack is on. |
| Uniform X-Spans | off *(build guess)* | On: columns evenly spaced. Off: column widths follow the 3D spacing (less texture stretch). |
| Uniform Y-Spans | off *(build guess)* | Same for rows. |

**UVT-33 Refusal.** Islands that aren't all quads, or are closed (a tube
with no cut), are left as they are, and a message says some islands
couldn't be straightened *(own wording)*.

---

## 6. UV transform (E6)

### 6.1 The existing tools in UV space

Extends UVE-1 §2.

**UVT-40** In a UV view, Move, Rotate, Scale, Transform and Element Move,
and the deform presets built on them (Smooth, Jitter, Shear, Taper, Pole),
work on U and V of the selected UVs. They use the usual pipe: action
centre, falloff, UV symmetry, snapping. Their handles are flat (U, V and a
centre handle).

**UVT-41 Tear Off.** A Transform option: when on, the selected polygons'
UVs are first split from their unselected neighbours (as Split, §9.1) and
then moved, so the piece comes away cleanly. Ctrl/Cmd+Shift held while
dragging turns Tear Off on for that drag.

**UVT-42 Seams move apart.** A UV position used only by selected polygons
moves. One shared with an unselected polygon on the far side of a UV
seam doesn't (UVE-1 §2).

### 6.2 Box Transform

**UVT-43** A UV-only tool. It draws a rectangle round the selected UVs and
lets you drag them:

| Drag | Result |
|---|---|
| Inside the box | Move. Ctrl constrains to U or V. Shift snaps the box corners to the main grid lines. |
| Rotation handle | Rotate about the pivot. Ctrl steps by Snap Angle (15° start). |
| Pivot handle | Moves the pivot. It snaps to the box's sides, corners and centre. |
| Right-drag a side | Scale from the opposite side (not from the pivot), on that axis only. |
| Right-drag a corner | Scale both axes together, from the opposite corner. |

Overlapping polygons show red while dragging (the UV view's overlap
overlay).

| Field | Effect |
|---|---|
| Offset U, V | Move amount. |
| Pivot U, V | Rotation centre. |
| Angle | Rotation about the pivot. |
| Box U, V | Box's lower-left corner. |
| Width, Height | Box size. Changing them scales the UVs. |
| Show Rotation Handle | Shows or hides the rotation and pivot handles. |
| Tear Off | As UVT-41. |
| Snap Angle | Step for Ctrl-rotate. |
| Gaps, Gaps by Pixel | Margin kept inside the tile (UVT-05). |

### 6.3 Radial Align and Linear Align

**UVT-44** The same tools as the vertex Radial Align and Linear Align
(unbuilt in Operandi in 3D too), run on UV positions: Radial puts the
selected UVs on a circle, with centre and radius worked out from the
selection and the first click. Linear puts them on a straight line.
Build once in the pipe, and they work in both kinds of view.

---

## 7. Island affine (E7)

One engine: **group** the target UVs, compute one **rule** per group, and
apply a move, turn, scale or flip. Each button below is one row.

| Grouping | Meaning |
|---|---|
| Entire | All target UVs as one block |
| Island | Each island (UVT-04) on its own |
| Polygon | Each polygon on its own |
| Edge row | Each run of connected selected edges on its own (the Alt+click forms) |

### 7.1 Fit UVs

Extends UVE-1 §3. Dialog: **Fit** (Entire UVs, per UV Polygon, per UV
Island), **Keep Proportion**, **Gaps**, **Gaps by Pixel**, **UDIM**
(the tile to fit into; 1001 start).

**UVT-50** Plain click: Fit Entire with the remembered values.
Shift+click: per Island, each island's bounding box fitted to tile 1001
(islands then overlap). Alt+click: per Polygon (the same as a Barycentric
layout).

### 7.2 Flip

**UVT-51** Flip U mirrors the selection left to right about its own
bounding-box centre. Flip V does the same top to bottom. **With Options**
adds **Per Polygon**: each polygon flips about its own centre. Alt+click on
Flip U or Flip V is a shortcut for Per Polygon.

### 7.3 Mirror

**UVT-52** Like Flip, but the mirror line is chosen: **Axis** U or V, and
**Center** (the line's position). Mirror U and Mirror V buttons use the
remembered Center. With Options opens the dialog. Center starts at 0.5
*(build guess, UVE-1 Q5)*.

### 7.4 Axis Align (Up, Down, Left, Right)

Extends UVE-1 §6.

**UVT-53** Up sets every selected UV's V to the selection's highest V.
Down sets it to the lowest V. Left sets U to the lowest U. Right sets U to
the highest U. Alt+click does this separately for each edge row. The full
dialog (Align UVs) adds Align To: Low, Average, Center, High.

### 7.5 Quick Align (Auto, Horizontal, Vertical)

**UVT-54** Rotates the whole island (not just the selection) so that the
first selected edge, or the line between two selected points, lies along
an axis. Auto picks whichever of U or V is nearer. Horizontal forces U.
Vertical forces V. The island turns about that edge's midpoint *(build
guess, UVT-Q06)*.

### 7.6 Island Align

**UVT-55** Moves each selected island so that, across all of them, their
left sides, right sides or centres line up (U choice: min, max, centre,
unchanged), and likewise their bottoms, tops or middles (V choice). The
toolbox shows the 16 combinations as a 4 × 4 grid of buttons. "Unchanged"
leaves that axis alone. Islands move, they don't turn or scale.

### 7.7 Orient Pieces

**UVT-56** Turns each island so it sits squarely. Auto picks the turn
giving the most compact layout. Horizontal lays each island's longer side
along U. Vertical lays it along V. Islands don't move or scale *(build
guess for "don't move")*. The engine crate already has the minimum-area
box turn Unwrap uses (UV-Q18 in the build notes).

### 7.8 To Origin

**UVT-57** Moves the selected islands so their lower-left corner sits on
the lower-left corner of a chosen UDIM tile (a small dialog asks which;
1001 start). Islands keep their size.

### 7.9 Texel Density and Real World Size

A small panel under Align and Pack.

| Control | Effect |
|---|---|
| Image Size | Texture resolution for the sums: a list of sizes (512, 1024, 2048, 4096 at least, plus user entries). Start 1024 *(build guess)*. |
| Get Image Size | From the first selected polygon's colour texture, from the selected texture, or from the render resolution. |
| Texel Density | Target, as pixels per unit with a unit choice. |
| World Space | On: item scale counts when measuring 3D size, so a scaled copy matches an unscaled one. |
| Sample Texel Density | Reads the current density of the selected islands into the target. |
| Apply Individually | Scales each selected island so its own average density equals the target. Alt: also turns islands upright. |
| Apply Overall | Scales all selected islands together so their combined average equals the target. Alt: also upright. |
| Normalize UVs | Scales the selected islands so they all have the same density (their combined average), with nothing else changed. |
| Real World Size: "1 UV = …" | The 3D length one UV unit stands for. Apply Overall here sets the UVs so one tile covers that length. |

**UVT-58** Scaling is about each island's own centre. Islands may overlap
afterwards: Pack is the next step if needed.

**UVT-59 Density sum.** Density of an island = Image Size × sqrt(UV area ÷
3D area). With World Space on, 3D area uses world-scaled points.

---

## 8. Pack

No change from UVS-1 §2. Rectangle's Pack option (§5.2) and Fit's Gaps use
the same engine and gap rule.

---

## 9. Cut and join (E9)

### 9.1 Split

**UVT-60** Splits the selected UVs from the rest: along selected edges,
round the edge of a polygon selection, or at selected points (split in
every direction). The pieces don't move, but each can now be moved on its
own. Shift+click: a large split, Ctrl+click: a medium one, Alt+click: a
custom distance in a dialog. These offset the two sides apart by that
distance *(build guess for the sizes, UVT-Q07)*.

### 9.2 Sew

**UVT-61** Joins matching pairs of separate UV positions (the same 3D point
on two islands) into one. Edges: selecting either side of a seam is
enough. Points: both copies must be selected. Dialog options:

| Connect to | Result |
|---|---|
| Average (button default) | Both sides move to the average position. |
| Center of Selected | The unselected side moves to the selected side. |
| Center of Unselected | The selected side moves to the unselected side. |

**Move Selected Only** keeps the unselected side still. Shift+click moves
the selected side onto the unselected side. Alt+click does the opposite.

### 9.3 Move and Sew

**UVT-62** Like Sew, but the whole island moves (and turns, and scales if
**Scale** is on, which it is to start) so the seam lines up before joining.
Connect to: Selected (the other island comes to this one), Unselected (this
island goes to the other), Average (both meet halfway). Shift and Alt are
shortcuts as for Sew.

### 9.4 Split and Sew brush

**UVT-63** A brush tool in the UV view. Mode **Split**: brushing over UV
points splits them apart along the connected edges, pushed apart by
Distance, direction per Split From (Source, Average, Destination). Mode
**Sew**: brushing over seam points joins each to its partner if the
partner is inside the brush too, per Connect To (Source, Average,
Destination). Shift held switches Split to Sew for that stroke.
Right-drag changes the brush size (pixels). **Move Part** (off) moves the
whole piece to meet the border. A stroke over one lone point splits it in
every direction.

---

## 10. UV symmetry (E10)

**UVT-70 UV Symmetry layer.** Off, U or V, plus **Center** (the symmetry
line's offset; 0.5 start, build guess). In a UV view, edits to a UV are
mirrored to its partner across the line. Partners are matched by UV
position, as 3D symmetry matches by position. With symmetry on, Center
(button) welds UV positions lying on the line *(guide: "joins the
discontinuous UVs at the line of UV symmetry")*.

**UVT-71 Symmetrize.** One-shot dialog. The mesh must be symmetric in 3D.

| Field | Effect |
|---|---|
| Symmetry Axis | X, Y, Z: the 3D mirror axis. |
| Direction | Positive to Negative copies from the + side to the − side. Negative to Positive is the reverse. |
| Mirror Axis | U or V: which UV value is mirrored. |
| Align to Symmetry Axis | Turns islands to line up with the mirror axis first. |

Each polygon on the target side gets its 3D mirror partner's UVs, mirrored
across the UV line. Uses Operandi's existing 3D symmetry pairing.

---

## 11. UV queries (E11)

**UVT-80 Select Border.** Selects the edges on every island's outline, in
the UV view and the 3D view together.

**UVT-81 Select Error UVs.** A dialog with check boxes. The polygons that
match any ticked test are selected:

| Test | Selects |
|---|---|
| Cross Intersection | UV polygons overlapping another UV polygon |
| Self Intersection | Twisted UV polygons (bow-tie shapes) |
| Flipped | UV polygons wound clockwise (facing away in UV) |
| Stacked | UV polygons exactly on top of another |
| Crossing UDIM Borders | UV polygons that cross a tile edge |
| Zero Area | UV polygons squashed to a line or a point |

**UVT-82 Select Unmapped.** Selects polygons with no UVs in the chosen map.
**Invert** selects the mapped ones instead.

---

## 12. UV values (E12)

**UVT-90 Cut, Copy.** Copy puts the selected polygons' UVs on a UV
clipboard. Cut does the same, then removes them from the map.

**UVT-91 Paste.** Writes the clipboard onto the selected polygons. Mode
**Connectivity**: matched by topology, so selection order doesn't matter
(same topology needed). Mode **Selection**: matched by the order the
polygons were selected.

**UVT-92 Transfer.** Copies UVs between meshes with the same topology in
one step: select two neighbouring polygons on the source, then the same
two in the same order on each target, then run. Transforms and deforms on
the targets don't matter.

**UVT-93 Delete.** Removes the selected polygons' UVs from the map (they
become unmapped).

**UVT-94 Set Value.** Sets U or V of the selected points to a typed value.

**UVT-95 Rotate Polygons.** Shifts each selected polygon's UVs one corner
along, which turns the texture 90° on a quad without moving anything in UV
space.

**UVT-96 Merge UV Maps.** Merges several maps of one mesh into the first
selected one, then deletes the others. Where a polygon has UVs in both,
the first map's UVs win.

---

## 13. Seams (E13)

### 13.1 Cut Map popover

Extends UVS-1 §1.6. Order: UV Seam (map), Add Seam, Clear Seam, Select
Seam (selects the seam map's edges), Delete Seam Map, Shortest Path
Selection (toggle for edge picking), Show UV Seam, Seam Color, Line Width,
then the Unwrap Tool, Unwrap & Relax and Seam Marker buttons.

### 13.2 Seam Marker

**UVT-100** A preset of the Conditional Loop selection tool, with its
action set to Seam Map: clicking an edge marks the loop it starts into the
seam map instead of selecting it. Conditional Loop's other settings (stop
rule, Loop Both Directions) apply. Conditional Loop isn't built in
Operandi yet: build it as a selection engine with an action choice
(Select, Selection Set, Seam Map), and Seam Marker comes with it.

**UVT-101 Seam map type.** A seam map is an edge map: an edge is either in
it or not. Operandi has edge selection sets already. A seam map can be
stored the same way, under its own type so it shows separately *(build
guess)*.

---

## 14. UV display (E14)

**UVT-110 Flipped tint.** An option to tint flipped UV polygons yellow
(blends with the red overlap tint).

**UVT-111 Coverage.** A percentage in the lower-right corner of the UV
view: share of the tile covered by UVs. With some items selected, their
share and the rest show separately. It updates when an edit ends.

**UVT-112 UDIM tiles.** Tile number = 1001 + U tile + 10 × V tile (U tiles
0 to 9). The view shows the tile number in each tile's centre when more
than tile 1001 is in view.

**UVT-113 UDIM Indicator.** A tool that draws a see-through frame on one
tile.

| Field | Effect |
|---|---|
| UDIM Number | Puts the frame on that tile. |
| Manual | Frees the frame: Position U, V, Width, Height set it. |
| Fit to Selection | (Manual only) Fits the frame round the selection. |
| Show Coverage | Coverage percentage inside the frame. |
| Select UVs in the Region | Selects all geometry fully inside the frame. |
| Move UVs in the Region | Asks for a tile, moves what's selected or fully inside there, and the frame follows. |
| Arrow buttons | Move the frame's contents one tile up, down, left or right. |

**UVT-114 Checker (Operandi addition).** The reference has no built-in
checker. Its own advice is to load a check image. Operandi adds a
**Checker** toggle in the UV view and in the 3D view's shading options: a
generated grey and white check, numbered per square, 8 × 8 per tile. This
is a new feature, not a match. It needs Jason's OK as a bible deviation
(UVT-Q08).

---

## 15. Export (E15)

**UVT-120** Export UVs to EPS and to SVG: save the current map's edges as a
vector drawing of the 0–1 tile. **Convert UVs to Mesh**: makes a new flat
mesh whose points sit at the UV positions (U to X, V to Y), one polygon per
UV polygon. Low priority. Painting apps read UVs from exported meshes
anyway.

---

## Acceptance cases

Plane with UVs (0,0) (1,0) (1,1) (0,1) unless stated.

- UVT-A01 Project Planar, Axis Y, Automatic, on a 2 × 1 m plane flat on the
  ground: UVs span the tile on the longer side, aspect kept *(fit rule
  build guess, UVT-Q09)*.
- UVT-A02 Project Barycentric on a cube: every face's UVs are (0,0) (1,0)
  (1,1) (0,1) in some corner order.
- UVT-A03 Cylindrical, Axis Y, on a 12-sided tube: U goes 0 to 1 once
  round, one seam, no polygon wider than 1/12 + a little in U.
- UVT-A04 Flip U on UVs spanning U 0.2 to 0.4: a point at U 0.25 goes to
  0.35.
- UVT-A05 Island Align min/unchanged on two islands with left sides at
  0.1 and 0.3: both left sides at 0.1, V unchanged.
- UVT-A06 Sew Average on an edge whose two sides sit at U 0.40 and 0.44:
  both at 0.42 and joined (one UV position).
- UVT-A07 Rotate Polygons on the plane: corners become (1,0) (1,1) (0,1)
  (0,0) or the reverse (UVT-Q10).
- UVT-A08 Texel Density, Image Size 1024, a 1 m square face with UV area
  0.25: density 512 px/m. Apply 1024: UV area becomes 1.0.
- UVT-A09 Select Unmapped after Delete UVs on one face: that face only.
- UVT-A10 Rectangle on a 3 × 2 quad patch, Uniform both on: a rectangle
  with evenly spaced 3 × 2 grid lines.
- UVT-A11 UDIM number of a UV at (2.5, 1.5): 1013.

## Open questions

- **UVT-Q01** Does Project show a 3D handle for its frame in Manual?
- **UVT-Q02** Starting values for Projection Type, Axis, Segments,
  Iterations, Orientation.
- **UVT-Q03** Falloff space for Relax in a UV view.
- **UVT-Q04** Peeler Uniformity start.
- **UVT-Q05** Snapping in the UV view: grid only, or UV points and edges
  too?
- **UVT-Q06** Quick Align's turning point.
- **UVT-Q07** Split's large and medium distances.
- **UVT-Q08** Checker overlay: OK to add as an Operandi extra?
- **UVT-Q09** Project Automatic: does it keep aspect or stretch to fill?
- **UVT-Q10** Rotate Polygons direction.

Q01, Q02, Q04, Q06, Q07, Q09 and Q10 can be read in one short bridge pass
when the bridge is free. Q05 needs a person at the mouse. Q08 is Jason's
call.
