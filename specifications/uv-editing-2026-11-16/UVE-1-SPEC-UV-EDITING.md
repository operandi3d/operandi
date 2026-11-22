# UV editing kit — behavior spec v1

Provenance: the user guide's UV pages (Create New Vertex Map, Delete UVs,
Cut/Copy/Paste UVs, View Projection, Fit UVs, Mirror UVs, Align UV,
Transform UV Map) for names, options and intent; the Cartoon Kid tutorial,
video 5 (iris, shirt, vest, trousers, belt, buckle, laces, soles, shoes,
skin, hair), and Whiskey Glass video 8 (rotate the table's UVs 90°) for
what is used. Not yet measured through the Modo bridge. REA-assisted: no.

Gap: Operandi has a UV view (selection, overlays), UV Unwrap (one-shot
dialog) and Pack UVs. It has no way to name and manage several UV maps, and
no way to move, rotate, scale, fit, mirror, align or project UVs. Kid video
5 can't be followed past the iris.

Order to build: §1 map list → §2 transform in the UV view → §3 Fit →
§4 Project from View → §5 Mirror → §6 Align → §7 copy/paste of a map.

## 1. UV maps per mesh

Each mesh can hold any number of named UV maps; one is the **selected**
(current) UV map, which every UV tool, the UV view and Unwrap act on.

A list of the active mesh's UV maps (the guide puts it under a Lists panel
as "UV Maps"; a section in the mesh's Properties also works):

| Action | Result |
|---|---|
| New Map | Asks for a name (default "Texture"; Q1 for repeats) and adds an empty UV map; it becomes the selected map. Polygons have no UVs in it until a tool writes them. |
| Click a map | Selects it. The UV view shows it; Image Map layers keep their own UV Map setting. |
| Rename | Double-click the name. |
| Delete | Removes the selected map(s) from the mesh; several can be selected and deleted at once (the tutorial deletes all the stray "Texture" maps in one go). Image Map layers that used it fall back to… (Q2). |
| Copy / Paste | Right-click Copy copies the whole selected map's values; selecting another map and Paste replaces that map's values with the copied ones (the tutorial: copy "Vest", New Map "Vest 2", Paste). Only meaningful within one mesh; Q3 for other meshes. |

Unwrap and the tools below write into the selected map; if the mesh has no
UV map, they make one called "Texture" first.

## 2. Move, rotate and scale UVs in the UV view

With a UV view active, the ordinary Move (W), Rotate (E) and Scale (R)
tools, and Transform (Y), act on the selected UVs in U/V instead of on
points in 3D:

- Selection is the UV view's (points, edges or polygons in the current
  mode, shared with 3D as today). Nothing selected = all UVs of the visible
  polygons.
- Handles are 2D: U axis, V axis and a centre handle for both (uniform for
  Scale). Rotate turns about the action centre (selection centre by
  default); Ctrl snaps rotation to 15° steps as in 3D.
- Fields show U, V offsets, one angle, U/V scale.
- Only the selected map changes; positions in 3D don't move.
- UVs shared by two polygons move together unless the UV edge is a seam,
  in which case each side's UV moves only with its own polygon's selection.

Tutorial uses: scale whole maps up to tile an image (shirt, trousers, belt,
laces), rotate an island 90° (table wood grain), drag islands on top of
each other so both shoes share one painted texture.

## 3. Fit UVs

Scales and moves the selected UVs to fill the 0–1 tile.

| Option | Default (Q4) | Effect |
|---|---|---|
| Fit | Entire UVs | Entire UVs: the selection as one block. Per UV Island: each connected island fitted on its own (islands end up overlapping). Per UV Polygon: each polygon on its own. |
| Keep Proportion | on (Q4) | Scale uniformly until the block touches the tile on its longer side; off stretches to fill both axes. |
| Gaps | 0 % | Margin left inside the tile, as a percentage of it. |

Tutorial: run once per iris island so each fills the tile.

## 4. Project from View (View Projection)

Writes UVs for the selected polygons as they appear in the view the tool is
clicked in: each point's UV is its position on screen, like a photograph.

- Activated as a tool; a click in a 3D view projects; clicking again after
  moving the view projects again.
- Auto Scale (on, Q4): the result is fitted into the 0–1 tile.
- Hidden Face (Q4): when on, polygons facing away from the view are left
  out.
- Writes into the selected UV map, or makes "Texture" if there is none.

Tutorial: front view, selected iris polygons, click → two flat discs, then
Fit UVs on each.

## 5. Mirror UVs

Flips the selected UVs across a line.

| Option | Effect |
|---|---|
| Axis | U (flip left–right) or V (flip up–down). |
| Center | Position of the mirror line on that axis (default the selection's centre, Q5). |

Tutorial: one sole and one shoe island mirrored so they sit exactly on top
of their partner.

## 6. Align UVs

Moves selected UV points onto one line.

| Option | Effect |
|---|---|
| Direction | Up/Down (moves V), Left/Right (moves U). |
| Align To | Low (box minimum), Average, Center (box centre), High (box maximum) of the selection's bounding box on that axis. |

Shortcuts the tutorial uses: "Align Down" etc., which flatten the selection
to its farthest point in that direction (Up = High V, Down = Low V,
Left = Low U, Right = High U). Used edge row by edge row to straighten the
belt and lace strips so a tiling image runs straight.

## 7. Copy/paste of UV positions between selections (later)

The guide also has Cut/Copy/Paste UV positions for selected geometry
(Connectivity and Selection modes) and Transfer. The tutorials don't use
them; leave for later.

## Acceptance cases

- UV-01 Plane with UVs (0,0),(1,0),(1,1),(0,1); Scale 2× about the centre
  (0.5,0.5) in the UV view → (−0.5,−0.5),(1.5,−0.5),(1.5,1.5),(−0.5,1.5);
  3D positions unchanged.
- UV-02 Same plane, Rotate 90° about the centre → (1,0),(1,1),(0,1),(0,0)
  (each corner moves one place, direction per Q6).
- UV-03 UVs spanning (0.2,0.3)–(0.4,0.9), Fit Entire, Keep Proportion on,
  Gaps 0 → spans V 0–1 and U width 1/3, placed at U start 0 (Q4).
- UV-04 Mirror U about Center 0.5: (0.1, 0.2) → (0.9, 0.2).
- UV-05 Align Down on points with V 0.3, 0.35, 0.28 → all V 0.28.
- UV-06 New Map "Iris" on a mesh with "Texture": two maps, "Iris"
  selected; Unwrap writes only into "Iris".
- UV-07 Copy "Vest", New Map "Vest 2", Paste: "Vest 2" equals "Vest";
  scaling "Vest 2" leaves "Vest" unchanged.

## Open questions

- Q1 Default name when "Texture" already exists.
- Q2 What an Image Map layer set to a deleted map shows.
- Q3 Pasting a whole map into another mesh with the same topology.
- Q4 Defaults of Fit UVs and View Projection options; where Fit places the
  block when Keep Proportion leaves room.
- Q5 Mirror Center default.
- Q6 Positive rotation direction in UV space (counter-clockwise expected).
- Q7 Whether 3D views can drive UV transforms (Modo has UV transform tools
  in 3D views too); not needed for the tutorials.
