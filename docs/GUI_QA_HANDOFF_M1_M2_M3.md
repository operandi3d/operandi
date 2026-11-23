# GUI QA handoff — Milestones 1, 2 and 3 (2026-10-31)

Owner: Codex (native GUI pass). Prepared by Claude Opus 5.5 for Jason.

This file lists every tool and command row implemented in M1 (Polygon), M2
(Edge) and M3 (Vertex), with the acceptance criteria to check in the running
app. All of it has passed the automated gate (fmt, strict clippy, 1164 tests
passed / 0 failed / 7 ignored, build). **No native GUI pass has been run for
the M1 fix pass, M2 or M3.** Earlier handoff items are folded in below.

## Where to test

- **Code:** worktree `~/Documents/Operandi-Rust-m3-vertex-tools`,
  branch `codex/m3-vertex-tools`, uncommitted. It contains M1 + the M1 fix
  pass + all of M2 + M3 (the M2 worktree has had no edits since this snapshot).
  Do not build or edit in the M2 worktree.
- **QA app (Jason approved this identity for M3):**
  `development/m3-vertex-gui-20261031/Operandi M3 Vertex QA.app`, bundle ID
  `org.operandi.rust.m3vertex.qa`, data in that folder's `AppData`. It holds
  the current debug build. It can't open `.opscene` from Finder; launch with
  `open -n -a "<app>" --args <scene path>`, or use File > Open inside the app.
- **Refreshing the app after a rebuild:** `cargo build -p operandi-app`, copy
  `target/debug/operandi-app` into `Contents/MacOS/`, then `xattr -cr` and
  `codesign --force --deep -s -` on the bundle.
- **Scenes** (before/after pairs made through the editor; compare your result
  with the `after` file):
  - M1: `docs/archive/m1-saved-demonstrations-20261030/` (removed 2026-11-05) and
    `docs/archive/m1-three-tools-20261030/demonstrations/` (removed 2026-11-05) (READMEs give selections).
  - M2: `docs/m2-demo/scenes/<row>.before|after.opscene`.
  - M3: `development/m3-vertex-gui-20261031/scenes/` (copies of
    `docs/m3-vertex-tools/scenes/`). The grid scenes lie in the XY plane: use
    Front view (Top shows them edge-on). Interior grid points are 5, 6, 9, 10.
- Never overwrite the user's installed Operandi or user scenes; Save As into
  the QA folder.

## Ground rules

- Read the current `AGENTS.md` first. Follow the public Modo 17.1 user guide
  over provisional specs; don't launch Modo or use private/REA material.
- No commit, push, merge or install.
- Report per item: **PASS / FAIL / BLOCKED / NOT RUN**, the steps, what you
  saw, and a screenshot path for each FAIL. One retry per inconclusive check.
  Write the report to `docs/GUI_QA_REPORT_M1_M2_M3.md` in the same worktree.
- Known: computer use in Claude Code classified both Operandi QA bundles as a
  browser (read-only), which is why this pass is handed to you.

## Checks common to every tool and row

Apply these to every item below unless the item says otherwise:

1. **Route:** the panel row, its shortcut (shown on the row and in its
   tooltip) and its command line entry all do the same thing. Tools:
   `tool.set <id> on|off|?` (`?` prints 1/0, changes nothing).
2. **Bad input:** an empty or invalid selection is refused with a status
   message; the mesh and Undo history don't change.
3. **History:** one Undo step per edit; Undo/Redo restore the whole scene.
   Live tools: Drop (Q) commits once; Esc/Cancel restores the scene from before
   the tool started.
4. **Save/reopen** keeps the result.
5. **Handles** follow the selection, including on a moved/rotated item.

## Milestone 1 — Polygon tab

IDs from `docs/M1_POLYGON_DELIVERY.md`.

### Shared modeling roles

| Item | Accept when |
|---|---|
| New Mesh (`N`, Scenes) | Creates and selects an empty mesh item. |
| Mesh Cleanup | Removes unreferenced (floating) points only. |
| Transform / Move (W) / Rotate (E) / Scale (R) | Component edits change the mesh; item edits change the item transform. Move X follows world X on a rotated item. |
| Scale center handle | A blue square where the three handles meet; dragging it right/up grows the selection uniformly. |
| Handles (all tools) | Bold lines, filled arrowheads, no text labels. |
| Box / primitives | Draw a box, then drag the blue height square below the ground: the box grows downward and stays solid (not inside out). |

### Interactive tools

| ID | Tool | Accept when |
|---|---|---|
| MP-T01 | Pen | Perspective and Top views: every click adds a point (even with slight wobble), drag moves a point, orbit then click stays on the same plane, Q drops a polygon visible from the camera. Adds to a non-empty mesh. |
| MP-T01b | Pen options | Type Polyline: three clicks, Q leaves an open line in click order; Close on joins it. Type Vertices: two clicks, Q adds two points. Flip Polygon on: the dropped polygon faces away. Select New off: selection unchanged. Greyed options show a reason on hover. |
| MP-T02 | Bevel (Shift+B) | Drag the inset square toward the selection center: the cap shrinks; away: it grows. Shift raises the cap. |
| MP-T03 | Extrude (Shift+X, Polygon mode) | Activate: no gizmo. Click viewport: gizmo appears, nothing moves (Drop leaves in-place walls). Drag an arrow: moves along it; drag elsewhere: moves freely. |
| MP-T04 | Chamfer Edit | Bevel a cube top (Shift and Inset > 0), Drop; select cap + four walls; drag the blue widget: the slanted ring widens all round. Segments 3: ring rounds, corners stay connected. Chains around corners are refused (known). |
| MP-T05 | Smooth Shift | On a cube top: Scale 70 with Offset 0 keeps the face on top (inset only); Offset alone moves without resizing. |
| MP-T06 | Thicken | Same field checks as Smooth Shift; produces a closed shell. |
| MP-T07 | Spike | Strength pulls a point out of each selected polygon's center. |
| MP-T08 | Poly Haul | Drag over a selected polygon: up pulls out, down pushes in, along the normal; no new edges. |
| MP-T09 | Sketch Extrude (Alt+B, provisional) | One primary drag gives a selected cap and connected walls; source section kept. |
| MP-T10 | Bridge | Two separate polygons on the same side of a plane, Curve mode: an arched bridge, not flat or broken. Unequal edge counts are refused (known). |
| MP-T11 | Polygon Reduction | On a sphere: Number lowers the polygon count while keeping the shape. |

### Command rows

Spot-check each from its row on ordinary cube/sphere selections: no refusal,
expected result, one Undo step.

| ID | Row | Accept when |
|---|---|---|
| MP-C01 | Make (P) | Two edges at a corner get their fourth point; two separate edges are bridged. |
| MP-C02 | Set Position | Dialog sets the chosen axis for the selection. |
| MP-C03 | Split (Ctrl+L) | Cuts between selected vertices, chaining across polygons. |
| MP-C04 | Triple (Shift+T) | Triangulates, including nonplanar polygons. |
| MP-C05 | Quadruple | Pairs triangles into quads. |
| MP-C06 | Spin Quad (Alt+V) | Any adjacent pair with equal corner counts, curved surfaces too. |
| MP-C07 | Flip (F) | Normals reverse. |
| MP-C08 | Align | Whole selected patch aligns to the first selected polygon. |
| MP-C09 | Merge | Several edge-connected regions merge in one edit. |
| MP-C10 | Remove (Shift+Backspace) | Polygons deleted. |
| MP-C11 | Collapse (Ctrl+Backspace) | Each connected piece becomes one point at its mean; neighbours heal. |
| MP-C12 | Unify | Removes duplicate polygons. |
| MP-C13 | Scale Offset | Insets the selection within its frame. |
| MP-C14 | Flatten Ngons | Flattens selected n-gons. |
| MP-C15 | Set Material (M) | Assigns a new or existing material, with colour and shading settings (see Set Material dialog below). |
| MP-C16 | Set Part | Assigns a part name. |

Known open issue (don't re-report): clicking Triple while a Move numeric field
has focus shows no status message.

## Milestone 2 — Edge tab

Full matrix: `docs/m2-demo/ACCEPTANCE_PACKET.md`. Edge mode for all items.

| Item | Accept when |
|---|---|
| Edge tab | Third toolbox tab "Edge" with the tool rows, icons (Edge Crease shows a marked placeholder) and the Commands section. |
| ME-T01 Edge Bevel | Select one cube edge, run the row, drag the handle: a strip appears and neighbours shrink. Round Level 2 rounds it. Unbound by default. |
| ME-T02 Edge Extrude (Shift+X) | One interior cube edge: the handle is a blue Extrude arrow plus a red Width line with a square end. Dragging only the blue arrow raises a ridge (Width starts at a tenth of the shortest side edge); the red square shrinks the neighbours. The cube's corners at the edge's ends disappear and each end is closed by one triangle. Each press-to-release is one undo step. Shift+X in Polygon mode still opens polygon Extrude. |
| ME-T03 Extend (Z) | Boundary edges of a plane: drag X/Y/Z, a new row of polygons follows; Drop selects the new outer edges; Z again extends further. Also on a moved/rotated item, and on a cube edge (a fin grows off it). |
| ME-T04 Bridge | Two separate quads, one facing border edge each, Bridge row, Apply: a quad joins them. |
| ME-T05 Slide (S) | Edge loop on a cube/grid: edges slide along connected edges, both directions, stop at the far end. |
| ME-T06 Edge Relax | Bevel an edge with Round Level 2 and Depth 0, Drop; select its row edges plus the outer two; Edge Relax rounds the rows; Offset widens. |
| ME-T07 Edge Chamfer (B in Edge mode) | As Bevel; Segments 3 rounds; keeps an even width on a slanted side. |
| ME-T08 Add Loop | Row on, click an edge: a loop is cut at the click (Ctrl-click: middle); one Undo step per click. Row again / Q / Esc / another tool turns it off, after which clicks select. |
| ME-T09 Add Point | As Add Loop but only a point; both polygons sharing the edge get it (no gap when moving it). |
| ME-T10 Edge Crease | Set Crease 0.6 (field or handle), Drop: creased edges drawn red, stronger with weight; survives save/reopen; reactivating shows 0.6. |
| ME-C01 Grow Quads (Shift+Q) | Plane border: a row of quads grows out. |
| ME-C02 Fill Quads | Border of a hole is filled with quads. |
| ME-C03 Set Position | Dialog sets the axis for the selected edges' points. |
| ME-C04 Split | Edges split so polygons separate there. |
| ME-C05 Split Normals | Refuses without a normal map. |
| ME-C06 Spin (V) | On a triangulated quad, the diagonal turns. |
| ME-C07 Join | Two edges: the first moves onto the second. |
| ME-C08 Join Averaged | Two edges meet at their average. |
| ME-C09 Collapse (Ctrl+Backspace) | Edge becomes a point. |
| ME-C10 Remove (Shift+Backspace) | Interior edge dissolves; polygons merge. |
| ME-C10 Delete an edge loop (2026-11-10) | Also: Delete on one point removes its four quads (eight-sided hole, no triangles); Delete on polygons leaves a gap; Item mode deletes the item. On a sphere, double-click an edge to select a loop, pointer over the viewport, press Delete (fn+Delete). The loop goes, the polygons either side merge, no points or stray edges remain; Cmd+Z brings it back. Backspace instead keeps the loop's points (and polygons' points). |
| ME-C11 Edges To Curves | Curves are created from the edges. |
| ME-C12 Unbevel | Two loops of a Polygon Bevel ring collapse back to a sharp edge. |
| ME-C13 Edge Subdivide (D) | Each selected edge gets a middle point. |

## Milestone 3 — Vertex tab

Full criteria: `docs/m3-vertex-tools/ACCEPTANCE_CRITERIA.md`; matrix and
provisional readings: `docs/m3-vertex-tools/ACCEPTANCE_PACKET.md`. Vertex mode
for all items. Scene names are `MV-<id>-<name>` in the M3 scenes folder.

| Item | Accept when |
|---|---|
| Vertex tab | Fourth toolbox tab "Vertex": seven tool rows (Create, Vertex Bevel, Vertex Extrude, Merge Tool, Weld, Tear, Vertex Slide), then a Commands section. Merge Tool, Weld, Tear and Slide show the marked placeholder icon (known artwork gap). Shortcuts show on rows: B, Shift+X, S, P, Ctrl+L, Ctrl+Backspace, Shift+Backspace. |
| MV-T01 Create | Row on (no selection needed); clicks in Front/Top views add points on the work plane; X/Y/Z fields edit the last point; Drop keeps them and selects them in order; Esc removes them. |
| MV-T02 Vertex Bevel (B) | Interior point with ≥3 polygons: Inset handle cuts the corner, one new polygon fills it. Inset 0 = no change. Round Level > 0 is refused (deferred). |
| MV-T03 Vertex Extrude (Shift+X) | Interior point: blue Extrude arrow plus red square Width handle. Dragging only the blue arrow moves the point along its normal with a ring of new points already at the starting Width, joined by triangles. |
| MV-T04 Merge Tool | Distance handle/field: selected points within Distance merge onto the most recently selected one; polygons heal. Drop commits, Esc restores. |
| WD-01..05 Weld (drag, 2026-11-06) | Row arms Weld. Press on any point (selected or not) and drag: the point and its edges follow the pointer; within about 15 points of another point it snaps there with a ring highlight. Release while snapped: the two merge at the destination's position, an edge between them collapses, polygons left with two corners vanish (one Undo step). Release away from every point: the point just moves there, at the depth it started (one Undo step). A click without dragging does nothing. Weld stays armed for the next drag. Esc mid-drag cancels that drag only; Q/Esc/another tool disarms Weld. Selection is unchanged. |
| MV-T06 Tear (provisional) | X/Y/Z handles: the corner of the polygon lying in the drag direction gets its own point and follows the drag; other polygons stay. Zero drag = no change. |
| MV-T07 Vertex Slide (S) | One Slide handle: the point moves along its best-aligned edge, stops at the far end; topology unchanged. |
| MV-C01 New Vertex... | Dialog X/Y/Z adds one point there and selects only it. Enter/OK commits; Esc/Cancel closes with no change. |
| MV-C02 Make Polygon (P) | ≥3 points selected in order make one polygon facing by the right-hand rule. |
| MV-C03 Show Vertices | Toggle: point dots drawn in Polygon and Edge modes too (provisional); no scene, selection or Undo change. |
| MV-C04 Split | A point shared by n polygons becomes n points; moving one polygon shows the gap. |
| MV-C05 Split Edge (provisional) | Edges between two selected points get a middle point. |
| MV-C06 Split Polygon (Ctrl+L) | Two points of one polygon: it splits between them. |
| MV-C07 Set Position... | Dialog sets the axis for the selected points. |
| MV-C08 Align... | Dialog (Axis, Uniform): middle points move onto the line from the first to the last selected point; Uniform spaces them evenly. Enter/Esc as above. |
| MV-C09 Merge... | Dialog (Automatic / Fixed + Distance): Automatic merges only coincident points; Fixed merges within Distance. Enter/Esc as above. |
| MV-C10 Join | All selected points weld into the last selected. |
| MV-C11 Join Averaged | As Join, at the average position. |
| MV-C12 Collapse (Ctrl+Backspace) | Point removed, its polygons merge into one (no hole). |
| MV-C13 Remove (Shift+Backspace) | Points and their polygons deleted. |
| MV-C14 Remove Edge | Edges between selected points dissolve. |
| MV-C15 Reorder Index... | Dialog Swap/Top/Bottom: point numbering changes, shape doesn't. Enter/Esc as above. |
| MV-C16 Sync Index Order (provisional two-step) | Select two adjacent polygons on the source mesh, run the row (status says captured); select the same two on the target mesh, run again: target's point order matches the source, shape unchanged. |

## Primitive and selection decisions (2026-11-01)

| Check | Expected |
|---|---|
| PS-01 Ctrl mid-drag | Start dragging a Cube without Ctrl, press Ctrl partway, keep dragging: the drag carries on unchanged (no freeze, no message). Same starting with Ctrl and letting go. |
| PS-02 Shift-drag next | Draw a Cube (or Ctrl-draw a Sphere/Cylinder), set Segments/Sides, then Shift-drag on empty space: the first stays, a second is drawn with the same settings. Undo removes them one at a time. |
| PS-03 Kept settings | Set Cube Segments 3, Drop, click Cube again: Segments is 3. Reset Tool Attributes puts it back to 1, and the next Cube starts at 1. Same with Sphere/Cylinder Sides/Segments. |
| PS-04 Shift+click | Shift+click adds points, edges and polygons to the selection in each mode. |
| PS-05 Fill | Select a ring of polygons around a cap, Shift+Cmd click a polygon inside: the inside fills in one click. |
| PS-06 Edge loop double-click | Edge mode, flat grid: double-click an inner edge: its whole loop is selected (status "Select Loop: N selected Edge"). Shift+double-click an edge across it: that loop is added, the first stays. Cmd+Z undoes only the added loop. Double-click on a cube edge: only that edge stays selected ("Edge loop stops here"). Vertex double-clicks pick one point as before (Polygon: see PS-06c); Shift+Cmd click in Polygon mode still fills. |
| PS-06b Loop from several edges | Edge mode, flat grid: double-click a row edge (its loop), Shift+click an edge across it, press Loop (or Shift+double-click that edge): both loops are selected. |
| PS-06c Polygon double-click selects connected | Polygon mode, a mesh with two separate pieces (e.g. two Boxes in one mesh): double-click a polygon on one: that whole piece is selected, the other stays unselected (status "Select Connected: N selected Polygons"). Shift+double-click the other piece: it is added, the first stays. Cmd+Z undoes only the added piece. A slow second click, or a paint drag that comes back, picks one polygon. Shift+Cmd click still fills. |
| PS-06d Shift+double-click empty space expands | A mesh with two separate pieces (e.g. two Boxes in one mesh). Polygon mode: click one polygon, then Shift+double-click empty space (off the mesh): every polygon of that piece is added, the other piece stays unselected (status "Expand Selection: N selected Polygons"); one Cmd+Z goes back to the single polygon. Vertex mode: select one point, Shift+double-click empty space: its whole piece's points are added. Edge mode, flat grid: select an inner edge (and one on the other piece), Shift+double-click empty space: each edge's full loop is added, the edges stay. With nothing selected, or in Item mode, nothing changes (components: status "Expand Selection: select components first"). Unchanged: plain double-click on empty space clears; Shift+double-click on a polygon or edge adds that piece or loop (PS-06, PS-06c); a slow second Shift+click and a Shift+drag on empty space don't expand. |
| PS-07 Primitive UVs | Draw a Cube, Sphere and Cylinder: each panel shows Make UVs ticked, and the mesh gets a "Texture" UV map. Untick before drawing: no shape appears; draw: no map. Untick, Drop, click the tool again: still unticked; Reset Tool Attributes ticks it. |
| PJ-01 New Project | File > New Project…, enter a new absolute folder and a name, Create: the folder appears with operandi.project.toml and the eight standard folders; the window title shows the project name; the open scene is untouched. |
| PJ-02 Save/Open defaults | With a project set, Save As on an untitled scene suggests <project>/scenes/Untitled.opscene; Open suggests <project>/scenes/. |
| PJ-03 Set/Recent Project | File > Set Project… on that folder works; on a plain folder shows "isn't a project" and stays open. File > Recent Projects lists it; choosing it sets it. |
| PJ-04 Relaunch | Quit and reopen: the last project is active again (title), no scene auto-opens. |
| FD-01 Native panels | File > Open shows the macOS Open panel in the project's scenes/ folder, listing .opscene and .lxo files; Save As shows the Save panel (asks before replacing); Set Project shows a folder picker; New Project's Choose… button picks or creates a folder. Cancel in any of them changes nothing. Unsaved-changes prompt → Save on an untitled scene saves through the panel, then continues. |
| PJ-05 Project presets | With a project set, File > Project Presets > Save Layout/Viewport Display/Input Map to Project writes files into its workspaces folder. Change your own settings, Set Project again: nothing changes. Load Project … restores the project's version and it stays after restart. Without a project the items are greyed out. |
| KB-01 Input Map editor | Open Input Map. Search "bevel", then Find by Key and press B: Bevel and Edge Bevel show. Click Move's shortcut, press R: a red banner offers "Remove it from Scale"; take it, Save. R now starts Move. Reopen, Reset All, Save: W is Move again. Esc while "Press keys..." only cancels the recording. |
| PS-06 Lasso visibility | Right-drag lasso over a closed box in Edge and Polygon modes: hidden back edges/polygons are skipped; middle-drag selects them too. On an open box seen from inside, unobstructed back-facing polygons are selected. |

## Symmetry (2026-11-02)

| Check | Expected |
|---|---|
| SYM-01 Menu | The context bar shows "Symmetry: Off"; its menu switches Off/X/Y/Z and the label follows. Offset field moves the plane. Select Symmetrical and Symmetrize are greyed while Off. |
| SYM-02 Mirrored pick and move | Cube at the origin, Symmetry X, Vertex mode: click one +X corner: its −X partner is selected too. Move with the gizmo: the two move as mirror images. Shift-click to deselect one: both go. |
| SYM-03 Select Symmetrical | Select two polygons on the +X side, run Select Symmetrical: the matching −X polygons are added. Undo removes only the added ones. |
| SYM-04 Cube | Symmetry X, draw a Cube off to the +X side: a mirror copy appears on the −X side as you draw. One Undo removes both. |
| SYM-05 Symmetrize | Nudge one +X point, Symmetrize Positive: its −X partner jumps to the mirror position. |

## Falloff (2026-11-02)

| Check | Expected |
|---|---|
| PRIM-08 Cone/Ellipsoid draw | Click Cone (or Ellipsoid), press in the viewport and drag: it grows from the press point; the panel fields follow. Apply or Drop keeps it. |
| ITEM-01 Drag parent | Drag a mesh row in the item list onto another: it's parented (indented) and doesn't move in the view. Drag it onto empty list space: unparented. |
| ITEM-03 Scene row | The Items list starts with a row named after the scene ("Untitled" when unsaved); meshes sit indented under it. Drag a child onto the scene row: it's unparented. The scene row's arrow folds all items away. |
| ITEM-02 Hide with children | Ctrl-click a parent's eye: it and all its children hide; Ctrl-click again: all show. Plain click: only the parent. |
| AL-01 Slide | Edge tab > Add Loop, press on an edge of a cube and keep holding: the loop appears; drag along the edge and it slides with a % beside the pointer; release keeps it. One Undo removes it. |
| IT-01 Negative Scale | Item mode, select a mesh, Scale, type X −100%: refused with a message. Tick Negative Scale and type it again: the mesh mirrors. |
| ACT-01 Automatic | Action Center menu > Automatic, Polygon mode with nothing selected, Rotate: the handle sits at the middle of the mesh's bounding box. |
| KEY-01 Defaults | Polygon mode B: Bevel. Edge mode B: Edge Bevel. Y: Transform. |
| PRIM-09 Caps and axis | Cylinder: set Cap Top to Radial and Cap Bottom to Quads, then Axis X: the drawn cylinder updates each time. Sphere: Sphere Mode Tessellated, Level 2: a triangle ball. Drop, start the tool again: the same settings. Reset puts them back. |
| POP-01 Cmd+click menu | Select a polygon, Command-click the cube: a menu with Modeling Operations; its submenu lists Bevel … Triple. Choose Triple: the quad becomes two triangles. In Items mode it shows Item Operations; Duplicate adds "Name (2)". |
| IT-02 Rotation order | Item Properties: set rotation 30/45/60, switch Rotation Order XYZ → ZXY: the angles stay, the cube turns. |
| IT-03 Ctrl+Shift drag | Items mode, Move, Ctrl+Shift drag the red arrow: a copy moves, the original stays. |
| SEL-05 Row by Shift+Cmd | Flat grid, pick a corner polygon, Shift+Cmd click the other end of its row: the whole row is selected. |
| AL-02 Modifiers | Add Loop on: Ctrl-click an edge cuts at 50%; Shift-click cuts where clicked; Command-click does nothing. |
| ITEM-04 Drag ghost | Drag an item row: a faded copy of it follows the pointer until you drop it. |
| HND-05 Free handle | Move: drag the white square where the arrows meet; the selection follows the pointer in the view plane. Rotate: the same square turns it freely. |
| HND-07 Drag on the model | Vertex mode, select a point, Move: press on the model away from the arrows (also right on another point) and drag; the selection follows the pointer in the view plane. |
| HND-08 Undo while active | Move: make two drags, press Cmd+Z: only the second is undone and Move stays active (handle shows). Cmd+Z again undoes the first; Cmd+Shift+Z redoes it. Same for Rotate, Scale, Transform. |
| HND-06 Click away | Move/Rotate/Scale live: press on empty space beside the model and drag; the handle jumps there and the selection moves, turns or scales about that point. |
| WS-01 Views and panels | See docs/WORKSPACE_LAYOUT.md checks V-01..05 (split, tear-off views) and P-01..04 (floating Toolbox, Items, Properties). |
| FAL-06 Handles | Linear falloff: a line with a Start and End handle; drag either and the tinted points update. Radial: centre and three size handles with outlines. With Move active, the move follows the new weights. |
| FAL-01 Menu | "Falloff: None" on the context bar; its menu switches None/Linear/Radial and the label follows. Switch Linear → Radial → Linear: each keeps the values you set. |
| FAL-02 Weighted move | Segmented Cube (8 segments in X), Linear falloff Auto Size X, no selection, Move up with the gizmo: the −X end moves fully, the +X end not at all, a ramp between. Ease-In/Ease-Out/Smooth change the ramp. |
| FAL-03 Radial | Radial, Auto Size, Move up: a dome; Size changes its reach. |
| FAL-04 Bevel | Linear falloff across a row of polygons, Bevel Shift: the shift tapers to nothing at End. |
| FAL-05 Refusal | With a falloff on, Extrude refuses with a message; set None and it works. |

## Selection (2026-11-02)

| Check | Expected |
|---|---|
| SEL-01 Tie | Cylinder with 8 sides: pick side polygon 0 then the opposite one, Shift+G: only one half of the ring between them is selected (not both). |
| SEL-02 Points | On a grid, pick two points in one row, Shift+G: the points between are added. Two points not in a row: nothing changes. |
| SEL-03 Loop | Pick one polygon on a grid, Loop: its column is selected. |
| SEL-04 Parent | Two meshes, move/rotate one. Right-click the other in the item list > Parent To, Keep Position > the first: it stays put and shows indented under it. Undo; Parent To (without keep): it jumps. Unparent puts it back. |

## Edge Bevel junctions (2026-11-02)

| Check | Expected |
|---|---|
| EBJ-01 L | Box, select two top edges meeting at a corner, Edge Bevel Inset 0.1: two strips meeting at one point on the top, no extra corner polygon; the 7 strip edges stay selected. |
| EBJ-02 Corner | Select the three edges at a box corner: the corner becomes a small triangle. With Round Level 1 it becomes three quads meeting in the middle. |
| EBJ-03 Loop | Select the four top edges: the top shrinks evenly, the sides are cut just below it. |
| EBJ-04 Grid | On a flat grid, bevel one interior edge: each end closes with a small triangle. Edge Chamfer leaves nothing selected. |

## Edge Bevel / Edge Chamfer options (2026-11-03)

Start each check from a fresh Cube (1 m) in Edge mode, one top edge selected
unless it says otherwise; type values in the panel and press Enter.

| Check | Expected |
|---|---|
| EBO-01 Width | Edge Bevel, Mode Width, Width 0.1: the strip is wider than with Inset 0.1 (each side moves about 0.14). Edge Chamfer Width 0.1: narrower (about 0.07). |
| EBO-02 Square | Edge Bevel Round Level 2, Shape Square: a sharp two-face corner through the old edge, not a round one. Sharp looks the same. Edge Chamfer Segments 3, Square: same shape, a small triangle at each end, the old edge stays selected. Three edges at a corner, Square: the box corner stays sharp, three quads meet at the original corner. |
| EBO-03 Corner rows | Select the three edges at a corner, Edge Chamfer Offset 0.1, Segments 3 then 4: the corner fills with a neat patch of quads (a small triangle in the middle for 3), no holes or flipped faces. |
| EBO-04 Border | Plane with 3×3 segments, select one edge that touches the outer border, Edge Bevel Inset 0.1: the strip runs to the border with no extra cap outside it. |
| EBO-05 Edge End | Same plane, select a T of edges in the middle, Edge Chamfer Segments 3: with Profile the open ends keep their point; with Triangle Fan the ends poke out slightly and fill with triangles. |
| EBO-06 Use Material | Tick Use Material, name "Trim", Drop: only the new polygons use "Trim". |
| EBO-07 Stop / Weld | Edge Chamfer Offset 1.2 on one cube edge: refused with "reduce it or tick Auto Weld". Tick Auto Weld: accepted; the two side faces disappear and the strip runs corner to corner. The four top edges at Offset 1.2: refused even with Auto Weld (the corners have nothing to stop on). |
| EBO-08 Parallel | Move one top corner out so a side slants, Edge Chamfer Offset 0.1: the strip is even; untick Parallel: it tapers like Edge Bevel. |
| EBO-09 Kept options | Drop, then start Edge Chamfer again: Shape, Segments and the ticks are as you left them, Offset is 0. Edge Bevel keeps its own. |
| EBO-11 Bend end | Cube with 2×2 segments, select the edge from a face centre to the middle of the cube's top edge, Edge Bevel: refused with a message, nothing changes. Edge Chamfer (Profile) still works. |
| EBO-10 Greyed | Mitering, Coplanar, Flatness, Sharp, Sharp Corner, Boundary Extend/Coplanar, Reverse X/Y are greyed out with a "Not measured yet" tooltip. |


## Preferences and Undo shortcut (2026-11-02)

| Check | Expected |
|---|---|
| PREF-01 Undo | Make a cube, press Cmd+Z: it's gone; Shift+Cmd+Z brings it back. Edit menu shows the chords. In a text field, Cmd+Z undoes the typing, not the scene. |
| PREF-02 Open | Cmd+, (or Edit > Preferences…) opens Preferences; Close or Esc shuts it. The viewport doesn't react while it's open. |
| PREF-03 Blender | Choose Blender style: middle-drag orbits, Shift+middle pans, Ctrl+middle zooms; Alt+left no longer orbits. Quit and relaunch: still Blender style. |
| PREF-04 Maya | Choose Maya style: Alt+left orbits, Alt+middle pans, Alt+right zooms. Operandi (native) puts back Alt+left / Alt+Shift+left / Alt+Ctrl+left. |

## Viewport display (2026-11-03, branch viewport-display)

Spec: specifications/viewport-display-2026-11-03/SPEC.md. Scene: two
meshes (e.g. a sphere and a cube), one selected.

| Check | Expected |
|---|---|
| VD-01 Styles | Display menu: pick each style in turn. Wireframe shows back edges and no fill; Solid flat; Shaded as before; Vertex Map grey (or the map's colors); Gooch warm/cool; Cel banded; Reflection mirror-like with a horizon. |
| VD-02 Alt-click | Alt-click Gooch: only the unselected mesh turns Gooch and the menu marks it "○ inactive"; the ✔ stays on the active style. |
| VD-03 Per view | Split View, change the left view's style: the right view doesn't change. Click into the right view: its own menu shows its own settings. |
| VD-04 Overrides | Make Inactive Invisible hides the unselected mesh in this view only; Make Inactive Same as Active draws it like the selected one. |
| VD-05 Overlay | Active Meshes ▸ Colored, opacity 40: dark faint lines; None: no wire, but selected polygons still outline. |
| VD-06 Toggles | Show Grid, Show Wireframe in Item Mode (Item mode, overlay None: wire appears), Show Verts (points in Polygon mode too), Show UV Seam on a cube (island borders in pink), Smoothing off (smooth sphere goes faceted). |
| VD-07 Presets | Choose Solid Wire: ✔ on it. Change the overlay opacity: "Solid Wire *". Choose it again: ✔ gone, the old look back. Save Viewport Preset… "Mine" with Active only; quit and relaunch: "Mine" is listed; Delete Preset… removes it. |
| VD-08 Shortcuts | Numpad7 sets Wireframe, Numpad4 Shaded, in the active view only. None of the above adds an undo step. |
| VD-09 Style polish | Gooch on a smooth sphere: small white highlight toward the upper left, dark rim along the silhouette (Perspective and Top alike). Preferences > Display > Colors > Display Styles: change Gooch Warm and Reflection Sky; the view updates at once; Reset returns the old look. With Gooch, then Reflection, drag out a new Cube and change an Ellipsoid's Sides: the preview draws in the same style as the scene, not plain grey. |
## Tweak (2026-11-03, slice TWK-1)

| Check | Expected |
|---|---|
| TWK-G01 Key and button | Press T (any selection mode), or Tools > Tweak: Tweak is active (the Properties panel says Tweak). T again or Q drops it. The selection doesn't change. |
| TWK-G02 Hover | Tweak live, move over a cube: near a corner the point lights up; along an edge the edge; elsewhere on a face the face's outline. Nothing changes until you press. |
| TWK-G03 Drag | Press on a point and drag: only that point follows the pointer and stays under it, in any view. Same for an edge (both ends) and a face (all its corners). Release: Cmd+Z undoes just that drag and Tweak stays active. |
| TWK-G04 No selection needed | Select one polygon, then tweak a different one: the one you grabbed moves; the selection stays as it was. |
| TWK-G06 Symmetry | Symmetry X on a cube centred on the origin, Tweak a +X corner: the −X corner moves as its mirror image (hover shows it in orange). A point on the plane slides along the plane only. |
| TWK-G05 Empty space | Press off the model and drag: nothing moves and the view doesn't turn. |
| TWK-G07 Range | Make a Box with 8 × 8 × 1 segments and look from Front. Tweak: set Range to 1 m and drag a point: nearby points follow less the further they are, Try Shape Preset Smooth: the falloff softens. Range 0: only the point moves. |
| TWK-G08 Right-drag Range | Tweak live, Range 0: right-drag to the right anywhere (on the mesh or empty space): a 4-circle yellow sphere grows from where you pressed and the Range field counts up; left shrinks it and stops at 0; up/down does nothing. Release: the sphere goes. Zoom in and repeat: the sphere grows about as fast on screen. Shift/Ctrl/Cmd make no difference. Cmd+Z puts Range back and Tweak stays. The status line reads "Right Click and Drag: Adjust Falloff Range". Hovering and left-dragging show no sphere. |
| TWK-G08 Mode | Mode Polygon: pressing right by a corner grabs the face, not the point. Mode Vertex: a press in the middle of a face grabs nothing. Polygon Center: the face moves with its center under the pointer. |
| TWK-G09 Connected Elements | Two separate cubes in one mesh, Range larger than the gap. Ignore: dragging a corner of one cube also pulls the other. Use Connectivity: the other cube stays put. Rigid Connections: the whole grabbed cube moves as one and Range greys out. |
| TWK-G10 Edge Loops | On a grid, Connected Elements Edge Loops, drag an edge: the whole row (or column) it runs along moves together. Drag a point or a face: only it moves. |

## Undo and Auto-Save preferences (2026-11-04)

| Check | Expected |
|---|---|
| UAS-G01 Pages | Cmd+, shows Defaults > Application and Defaults > Auto-Save in the page list. Application shows Max Undo Level 100, Group Selection Commands off, Max Sub-Commands Recorded 100 (greyed until grouping is on). |
| UAS-G02 Undo limit | Set Max Undo Level to 3, close. Make a cube, click four polygons one by one, then Cmd+Z repeatedly: only three clicks come back. Quit and relaunch: still 3. |
| UAS-G03 Grouping | Turn on Group Selection Commands, Max Sub-Commands 3. Click five polygons one by one: Cmd+Z once removes clicks 4–5, again removes 1–3. Bevel something between clicks: the clicks before and after undo separately. |
| UAS-G04 Countdown | Auto-Save: Enable on, interval 1, Browse… to a test folder. Edit the scene; within a minute "Auto-save in 10 s" with Cancel shows at the top of the 3D view, counting down, then the status bar says Auto-saved and the folder has `<scene>.autosave-0001.opscene`. Your own scene file is unchanged. |
| UAS-G05 Cancel and revisions | Press Cancel during a countdown: no file that time; the next countdown comes a minute later. With Number of Revisions 2, after three backups only the newest two are in the folder. An unchanged scene gets no countdown. |
| UAS-G06 Remapping | Input > Remapping: Invert Mouse Wheel Zoom on, scroll up over a view: it zooms out. Acceleration 300 %: wheel zoom is clearly faster. Wheel Zoom at Mouse Cursor off: zoom stays on the view centre. Zoom Drag Axis Left to Right: Alt+Ctrl+drag right zooms in. Tooltip Delay 0: toolbar tooltips appear at once. Invert Mouse Wheel Scrolling: the Scenes list scrolls the other way, the 3D view zoom doesn't change. |
| UAS-G07 Selection | Input > Selection: Point Size 8 in Vertex mode: dots are clearly bigger and easier to click. Rollovers None: nothing lights up under the pointer. Hit Size 3: a click must land right on a point or edge. Initial Selection Type Edges, quit and relaunch: the new scene starts in Edges. Polygon Selection Frontface & Backface: a lasso across a solid cube picks back polygons too. Allow Partial on, relaunch: the lasso's Partial toggle starts on. |
| UAS-G08 Tool Handles | Display > Tool Handles with Move active: Handle Scale 2 doubles the arrows; Line Width 6 makes them thick; Selected Tool Handles Invisible removes them (the tool still works from its properties). |
| UAS-G09 Units | Input > Distance Units: Default Unit mm, open Cube: sizes show as 1000 mm; type 250 in Size X: the cube is a quarter metre. English: 42 inches shows 3' 6"; typing 2 ft works. Back to SI m: fields show metres again. |
| UAS-G10 Grid and Fit | Grid Type Fixed, Size 250 mm: grid squares stay 250 mm while zooming in; zoom far out and the grid coarsens instead of crowding. Fit Factor 1: Fit makes the cube nearly touch the view's top and bottom; 3 leaves a wide margin. |
| UAS-G11 Overlays and defaults | Display > 3D Information Overlays, all on: the lower left of the view lists the selection count, polygons drawn (grows when you Tab to subdivide), grid size, polygon type and the tool. Defaults > Tool Settings: component center Selection, then File > New: the action centre shows Selection. Scene Save/Export: Author and Copyright, export glTF: open the .gltf in a text editor and find both under "asset". |

## Multiple open scenes (2026-11-04)

| Check | Expected |
|---|---|
| MS-G01 New and switch | Make a cube, then click "(new scene)" at the bottom of the Items list: the list shows "Untitled" (plain) and "Untitled (2)" (bold), each a parent with its own items under it; scene rows cannot be dragged or parented. Click the "Untitled" row (or one of its items, which also selects it): it turns bold, the cube is back with its selection and camera; Cmd+Z there undoes only that scene's steps. File > Current lists both, the current one checked. Right-click a scene row: Save, Save As…, Export As, Close Scene act on that scene. |
| MS-G02 Tool stays | Arm Cone and type 7 sides (don't press Enter), File > New, then click the first scene's row: Cone is still armed with 7 in the field. |
| MS-G03 Open | Save a scene, Cmd+W it (you get a blank Untitled), then File > Open it: it fills the blank scene, no extra tab. Open it again from another scene: it just switches to it. |
| MS-G04 Close Scene | Cmd+W on an edited scene: "Save … before closing it?" with Save/Discard/Cancel. Cancel keeps it; Discard closes it and the next scene becomes current. Cmd+W on the last scene leaves one blank Untitled. |
| MS-G05 Close All | Two edited scenes and one clean: Cmd+Shift+W asks "Close all open scenes?"; Close All asks about each edited scene in turn and ends with one blank Untitled. |
| MS-G06 Quit | Two edited scenes: Cmd+Q asks about the first, Discard, then about the second; Cancel there: nothing is lost in either (check the first scene still has its edits). Quit again discarding both: relaunch shows no recovery prompt. |
| MS-G07 Recovery | Two edited, unsaved scenes, then force-quit (Activity Monitor). Relaunch: the recovery prompt says 2 scenes; Recover opens each as its own scene. |
| MS-G08 Auto-save | Auto-save on, interval 1, two edited untitled scenes: backups named `Untitled.autosave-…` and `Untitled (2).autosave-…` appear; switching scenes backs up the scene you leave if it changed. |

## Backdrop items (2026-11-04, branch backdrop)

Use any PNG or JPEG with an obvious top-left (a photo or a labelled drawing).

| Check | Expected |
|---|---|
| BD-G01 Create and load | Items list > Add Item > Backdrop Item: "Backdrop Item" appears, selected, as a 1 m orange outline round a see-through orange grid in the Front and Perspective views, and Properties shows a Backdrop section in this order: Image, Show In Perspective, Projection Type, Keep Aspect, Flip, Invert, Blend, Contrast, Brightness, Transparency, Use Uniform Size, Pixel Size, Auto Fit, Width/Height with Backdrop Size and Backdrop Size Keep Aspect. Image > Load Image…, pick the picture: in a Front view it shows the right way round, centred on the origin, covering the grid; a mesh in front of or behind it always draws over it. Its width in metres is pixels × Pixel Size (10 mm). |
| BD-G02 Views | Split View into Front, Top, Back and Perspective: the picture shows in Front and Perspective only. Back shows nothing; set Projection Type to Back: it shows in Back, mirrored. Show In Perspective off hides it in Perspective only. Display > Show Backdrop off in one view hides it there only. The eye in the Items list hides it everywhere. |
| BD-G03 Options | Flip mirrors the picture inside the same rectangle. Invert, Contrast +50 %, Brightness −50 % change the colours; Transparency 50 % lets the background and grid show through. Blend on: zoom in on a small picture and the pixels are smoothed instead of hard squares. Drag a number field: one undo step per drag. |
| BD-G04 Size | Use Uniform Size on: the picture becomes 1 m tall and Pixel Size greys out (hover says why). Keep Aspect off: it becomes a square. Item Mode, Scale only X: nothing changes until Y is scaled too (Keep Aspect on). Width 3, Height 2, Backdrop Size: exactly 3 × 2 m; Backdrop Size Keep Aspect with width 4: 4 m wide, picture shape kept. |
| BD-G05 Auto Fit | Make a cube, select the Backdrop in the Items list, turn Use Uniform Size on, Auto Fit: it moves to the cube's front face and is as tall as the cube. With Uniform Size off Auto Fit is greyed. |
| BD-G06 Move and frame | With the Backdrop selected, in any selection mode: Transform, Move, Rotate and Scale switch to Items mode and move it like any item; a one-pixel outline in the selection colour surrounds the selected picture only. Component tools refuse with a message. Frame All in Front includes the picture. Clicking the picture in a view doesn't select it (provisional). |
| BD-G07 Save and missing file | Save, close, reopen: every setting and the picture come back. Quit, rename the picture file, reopen: the scene opens, the picture doesn't draw, and Properties shows "Backdrop picture not found" in red. |
| BD-G08 Commands | In the Command field: `backdrop.create`, then `backdrop.attr projection top`, `backdrop.attr contrast 50`, `help backdrop.attr`: each does what the panel does, and Undo steps back one at a time. |

## Cameras and lights CL-1 (2026-11-04, branch cameras-lights-cl1)

| Check | Expected |
|---|---|
| CL-G01 New scene | File > New: Items shows Mesh 1, Camera, Directional Light (camera and sun glyphs, eyes); Mesh 1 selected. No camera or light is drawn in any view, even when selected. Display > Show Cameras and Show Lights on: the camera shape appears at (0, 0.75, 4) looking slightly down toward the origin, and the light disc at (−2, 2, 2) with an arrow pointing down toward +X/−Z; selecting each in Items turns its shape the selection colour. |
| CL-G02 Properties | Select Camera: a Camera section (Projection, Focal Length 50 mm, Angle of View ≈ 39.6°, Film 36 × 24 mm, Offsets, Film Fit Fill, Target Distance 4 m, Clipping (Clipping Distance 4 m), Make Render Camera greyed with a tick). Focal Length 25: Angle of View ≈ 71.5°, frustum widens. Directional Light: Render, Dissolve, Intensity 3, Shadow Type, Visible to Camera (off), Spread Angle, Color, Affect Diffuse/Specular, and a closed 'Not used yet' section (Samples 64 and so on). Each change is one Cmd+Z. |
| CL-G03 Item mode only | With Camera selected in Polygon mode: Bevel/Extrude refuse with a message; Move switches to Items mode and moves the camera. |
| CL-G04 Camera view | View menu > Camera: the view looks through the camera (origin centred). Alt-drag orbits, Shift+Alt pans, Ctrl+Alt zooms: on release the camera shape moves in a second Perspective view (Show Cameras on there); one Cmd+Z per drag puts it back and the Camera view follows. Projection Orthographic: the Camera view goes flat. |
| CL-G05 Show toggles | Both start off. Show Cameras on in one view draws cameras there only; same for Show Lights. Selecting a camera with the toggle off still draws nothing. Camera view never draws the camera it looks through. |
| CL-G06 Render | (Cycles build) Make a cube, File > Render Viewport: a 1920 × 1080 picture from the camera, lit by the directional light. Light Render Off: the picture has only sky light. Delete camera and light: renders from the viewport as before. |
| CL-G07 Save/open | Save, reopen: camera/light settings and the render camera come back. Open an older scene: no camera or light is added. |

## Point, Spot and Area lights CL-2 (2026-11-04, branch cameras-lights-cl1)

| Check | Expected |
|---|---|
| CL-G08 Create | Item > Create Light > Point: "Point Light" appears at the origin, alone selected, drawn as a small star (with Show Lights on); Cmd+Z removes it in one step. Same for Spot (cone pointing down −Z with an inner ring) and Area (1 × 1 m square with a short normal line). Item > Create Camera adds "Camera (2)" at the origin. |
| CL-G09 Properties | Point: Intensity labelled Radiant Intensity (W/sr), Radius, Falloff Type (Inverse Distance Squared). Radius 0.5 draws three circles. Spot: Cone Angle 45°, Soft Edge 5°; Cone Angle 80° widens the drawn cone. Area: Shape Elliptical draws an ellipse; Width 2 doubles it; scaling the item in Item mode stretches it too. Intensity labelled Radiance. |
| CL-G10 Render | (Cycles build) A cube with only a Spot light 3 m above it, rotated to point down: the render shows a soft-edged pool of light. Falloff Type None: the status line notes Cycles uses inverse square. |

## Cameras and lights v3/v4 (2026-11-05, branch cameras-lights-cl1)

Turn Display > Show Cameras and Show Lights on first.

| Check | Expected |
|---|---|
| CL-G11 Glyphs | Create one light of each kind. Light grey shapes: camera body with reels and a lens toward −Z (same size on screen however far you zoom), directional cylinder with a filled arrow, point star, spot lamp housing, area square with a dotted cross and a filled arrow. Select each: it turns orange; the camera adds a dotted frustum, the spot a dotted cone. |
| CL-G12 Items list | Each row has its own glyph. The render camera's name is bold with a ring-and-dot marker; Set as Render Camera on another camera moves both. |
| CL-G13 Tool channels | Select the Area Light, press W (Move): Tool Properties shows Width and Height under the tool's fields; change Width to 2 and the square widens about its centre. Cmd+Z takes the Width back with Move still active; Escape restores it. Spot shows Cone Angle/Soft Edge, camera Focal Length/Focus Distance, point Radius, directional nothing extra. |
| CL-G14 Handles | With Move active: area light shows cyan squares mid-right and mid-top; dragging the right one widens both sides. Spot shows a purple circle and a dotted inner circle down its axis with cyan crosses; dragging the right cross changes Cone Angle, the top one Soft Edge. Camera shows a cross at Focus Distance on its axis; drag along the axis. Point (Radius 0) shows a centre cross; drag out to grow three purple circles. Each drag is one Cmd+Z inside the tool. |
| CL-G15 Camera view follows selection | Add a second camera and select it: View > Camera looks through it and the header says "Camera (2)". Select the first: the header says "Camera · Render Camera". |

## Drag-to-scrub numeric fields (2026-11-04)

| Check | Expected |
|---|---|
| NS-G01 Handle | Activate Bevel on a cube polygon: every numeric field in Tool Properties has a small ◂▸ handle on its left; hovering it shows a left-right cursor and a tooltip. Rows are no taller than before. |
| NS-G02 Scrub live | Drag Inset's handle right about 100 points: the value climbs about 1 m and the viewport bevel follows every frame. Shift-drag moves ten times faster, Option-drag ten times slower. Release, then Drop, then Cmd+Z once: the whole bevel goes. |
| NS-G03 Counts and limits | Drag Segments left past 1: it stops at 1 with no red error; dragging back right raises it straight away, in whole steps. |
| NS-G04 Escape | Mid-drag press Escape: the value and preview jump back to where the drag started, and the tool stays active. |
| NS-G05 Move steps | Move a polygon by scrubbing Position Y, release, scrub again, release: Cmd+Z undoes only the second scrub, then the first. The field reads 0 after each release (like after a handle drag). |

## Torus, Capsule, Arc, Tube (2026-11-04)

Spec: specifications/new-primitives-2026-11-03. Values are set in the tool
panel; drawing in the viewport isn't built yet (NPR-Q01).

| Check | Expected |
|---|---|
| NPR-G01 Torus | Create tab, Primitives: Cube, Sphere, Ellipsoid, Cone, Cylinder, Torus, Capsule; Draw: Arc, Tube in the same cells (collapsible). Click Torus. A torus previews at once; the panel shows Ring Radius 0.5, Cross Section 0.25, Height 0.25, Hole Size 2. Type Hole Size 1 and Enter: Ring Radius becomes 0.25 and the hole closes. Type Radius X 1: Cross Section 1 and Ring Radius 2. Drop: one "Torus" undo step, nothing selected. |
| NPR-G02 Capsule | Click Capsule, set Sides 8, Segments 2, End Segments 2, then Ends Quad Grid: each end is 4 quads round a centre point. Axis X turns it. Apply adds it and keeps the tool; Cancel removes the unapplied preview only. |
| NPR-G03 Arc | Create tab, Draw section (below Primitives): click Arc: a quarter-disc wedge in the XY plane. Angle 270 shows 180. Reverse on: the rest of the disc, facing the other way. End 0, 0, -1 (type each field): the wedge lies flat toward -Z. |
| NPR-G04 Tube | Draw section: click Tube: no preview until two points exist. Add Point twice: a straight tube up Y. Add a third point and set it to 2, 0, 0: the tube bends smoothly through it. Closed on: the ends join and Caps greys out. Twist 90: the ring corners turn along the tube. |
| NPR-G04b Tube clicks | Click Tube, then click three spots in the viewport: each click adds a point (status counts them) and the tube runs through all three, not restarting at the last click. Drag from a fourth spot: a new point follows the pointer. Cmd+Z removes the last point; the tool stays. |
| NPR-G05 Menus and switching | Item > Primitive lists Torus, Capsule, Arc, Tube, checked while active. With Torus showing, click Capsule: the torus is added and Capsule starts. Click Capsule again: it drops. |
| NPR-G06 Save and reopen | Make one of each, save, reopen: all four come back the same. |

## Undo while a tool is active (2026-11-04)

Each tweak inside a tool is its own undo step; a dropped tool is one step.

| Check | Expected |
|---|---|
| MTU-G01 Cube Z drag | New Mesh, Cube. Drag out a box, then drag the blue (Z) handle. Cmd+Z: only the Z drag goes back; the box and the Cube handles stay. Shift+Cmd+Z: the Z drag comes back. Q, then Cmd+Z: the whole box goes in one step. |
| MTU-G02 Undo to empty | Cube, draw a box, Cmd+Z: the box goes, Cube stays active. Cmd+Z again: Cube closes, nothing else changes. Shift+Cmd+Z twice: Cube opens, then the box comes back. |
| MTU-G03 Move | Cube, select its top polygon, W, drag Y up, then drag X. Cmd+Z: only the X drag goes back, handles stay. Cmd+Z: the Y drag goes back. Cmd+Z: Move closes, the face stays selected. Redo three times brings it all back; Q then Cmd+Z undoes both drags at once. |
| MTU-G04 Move scrub | With Move, scrub the Y field twice. Each Cmd+Z takes back one scrub. Escape: the face goes back to where it started. |
| MTU-G05 Cone | Empty mesh, Cone, Sides 8, Apply. Type Sides 12, Enter. Cmd+Z: Sides back to 8. Shift+Cmd+Z, Apply. Cmd+Z: the second cone goes, Sides 12. Cmd+Z: the first cone goes, Sides 8. Cmd+Z: Cone closes. |
| MTU-G06 Escape and Q | Two cones as in MTU-G05, Cmd+Z once, then Escape: Cone closes, the first cone stays, Edit > Redo is greyed. Same with Q. |
| MTU-G07 Torus | Click Torus, Apply, change Sides: Cmd+Z puts Sides back, the panel stays open. Cmd+Z again removes the torus. |
| MTU-G08 Bevel | Cube, select the top polygon, Bevel. Drag Inset, then drag Shift. Cmd+Z: only the Shift drag goes back, Bevel stays. Cmd+Z: the Inset goes back. Q after redoing both, then Cmd+Z: the whole bevel goes at once. |
| MTU-G09 Other tools | Repeat MTU-G08's two drags and one Cmd+Z with Smooth Shift, Edge Bevel, Vertex Bevel and Tweak: each Cmd+Z takes back one whole drag and the tool stays. |

## Oracle replay fixes (2026-11-04)

| Check | Expected |
|---|---|
| ORF-G01 Extrude caps | New Mesh, Cube with Size Y 0 (a flat quad). Select it, Extrude, drag up, Q. Both the bottom and the top polygon are selected. |
| ORF-G02 Extrude nothing selected | Same quad, nothing selected, Extrude, drag up: the quad extrudes. |
| ORF-G03 Subdivide | Cube, Edge mode, select one edge, Subdivide. Switch to Vertex mode: the new point is selected. |
| ORF-G04 Vertex Join | Cube, select three points, Join: nothing stays selected. |
| ORF-G05 Merge | Two adjacent polygons, Merge: the merged polygon is selected. |
| ORF-G06 Zero Bevel | Select a polygon, Bevel, press Apply without dragging, Q: the polygon count goes up by 4 and nothing looks broken. |
| ORF-G07 Negative Cube | Cube, type Size X -1: the same cube as Size X 1. |

## Smooth shading (provisional spec)

Spec: smooth-shading v0 (2026-11-03, read from documentation, not observed).
The settings are in the item properties under Smoothing and apply to every
mesh.

| Check | Expected |
|---|---|
| SS-G01 Defaults | New scene, add a Cube and a Sphere. Smoothing shows 100 %, Smoothing Angle 40°, Crease by Smoothing Angle on, Smoothing Groups Crease. The cube's sides are crisp; the sphere looks round; its silhouette stays faceted. |
| SS-G02 Angle | Drag Smoothing Angle to 100°: the cube's shading rounds over its corners. Release, then Undo once: back to 40° in one step. |
| SS-G03 Amount | Type Smoothing 0: everything faceted. 50: the sphere is half way. 200: overshoot ("pillowing"). |
| SS-G04 Viewport switch | The view's Smoothing switch off: all faceted whatever the settings; back on restores them. Not an undo step. |
| SS-G05 Drag | Sphere at defaults, Move a few points: shading follows smoothly with no stale dark corners next to the moved points. |
| SS-G06 Save and reopen | Set 75 % / 25°, save, reopen: the values and the look come back. |
| SS-G07 Render | Render (Cycles) the cube and sphere at defaults: cube crisp, sphere smooth. |

List the FAILs at the top of the report with their scene, steps and
screenshot. Fixes go back to Claude in the M3 worktree; don't change code
during the pass.

## Drag anywhere (2026-11-04)

Each check: arm the tool on a suitable selection, then press well away from
the mesh and its handles and drag. Release, Undo once: only that drag is
undone and the tool is still active.

| Check | Expected |
|---|---|
| DA-G01 Bevel | Polygon selected. Drag up: the cap rises (Shift). Drag right: the cap grows; left: it shrinks. Dragging the Shift or Inset handle still changes only that value. |
| DA-G02 Smooth Shift / Thicken | Click once to start. Drag up: Offset grows; right: Scale grows. |
| DA-G03 Edge and Vertex Extrude | Drag diagonally up-right: both Extrude and Width grow. |
| DA-G04 One-value tools | Chamfer, Edge Bevel, Edge Chamfer, Edge Relax, Edge Crease, Vertex Merge: drag right grows the value, left shrinks it. |
| DA-G05 Slides, Extend, Tear | Edge/Vertex Slide follow the rail; Edge Extend and Vertex Tear follow the pointer in the view. |
| DA-G06 Poly Haul | Drag up from empty space: the selected polygons pull out. |
| DA-G07 Sphere / Cylinder | Draw one, then drag away from it: the nearest side handle resizes. |
| DA-G08 Spike, Bridge, Reduction | Spike: up lengthens. Bridge (two loops selected): right adds segments. Reduction: press initializes, drag left reduces. |
| DA-G09 Torus, Capsule, Arc, Tube | Arm, then drag on the grid: the shape is drawn from the press, sized by the drag. |
| DA-G10 Modifiers | Ctrl-drag off the handles with Bevel: nothing changes. |

## Selection Sets (2026-11-04)

Use the command line until the Select tab buttons arrive. Cube, Polygon
mode.

| Check | Expected |
|---|---|
| SS-G01 Assign | Select two polygons, `selset.assign` opens the dialog; type Top, OK. Status says Assigned. Undo once: the set is gone (Edit menu says Undo Assign Selection Set). |
| SS-G02 Use | Clear the selection, `selset.use Top`: the two polygons are selected. `selset.use Top remove` deselects them. |
| SS-G03 Modes | In Vertex mode `selset.use Top` says there is no vertex set Top. Assign a vertex set there; it does not show in Polygon mode. |
| SS-G04 Edge sets | Edge mode: select an edge loop, `selset.assign Loop`, select nothing, `selset.use Loop`. |
| SS-G05 Group and delete | `selset.group` with two sets ticked, OK. `selset.delete` picks one set to delete; the geometry stays. |
| SS-G06 Save/reopen | Save the scene, reopen: the sets come back. |

## Modifier-key buttons (2026-11-04)

Cube, Polygon mode, one polygon selected, mouse over the viewport.

| Check | Expected |
|---|---|
| MB-G01 Option | Hold Option: Vertices/Edges/Polygons read Convert, the lit Polygons button goes unlit, Set Material... reads Set Part.... Let go: all back at once. |
| MB-G02 Shift/Ctrl | Shift: only Edges changes, to Bounds. Control: only Edges, to Boundary. Cmd: nothing changes. |
| MB-G03 Click | Option-click Vertices: the polygon's 4 points are selected in Vertex mode. Press Vertices, then hold Option before letting go: a plain mode switch. |
| MB-G04 Pair | Hold Shift+Option: Edges reads Edges; clicking switches mode only. |
| MB-G05 Tooltip | Hover Edges until its tooltip shows; hold Shift: it changes to Bounds. Its second line lists Shift/Ctrl/Option variants. |
| MB-G06 Typing | Click into the command field, hold Option: the buttons still swap. |
| MB-G07 App switch | Hold Option, Cmd-Tab away, let go there, come back: buttons show their normal labels. |
| MB-G08 Bounds/Boundary | Select two neighbouring polygons of a cube. Shift-click Edges: 6 outline edges, Edges mode. Select one other edge, back to Polygons, Ctrl-click Edges: that edge stays, the 6 are added. |
| MB-G09 Primitives | Create tab. Hold Shift: every primitive cell shows "Item", tooltip "Cube Item"; Shift-click Cube: a new 1 m cube item, no drag. Hold Control: "Unit"; Control-click Sphere: a 1 m sphere in the current mesh. Press Cone with Shift, let go of Shift before the mouse: still a Cone item. |
| MB-G10 Primitives while typing | Click into the command field, hold Shift: the cells still show "Item". |
## Set Material dialog (2026-11-04)

Select some polygons of a cube in Polygon mode, then press M (or Set
Material... in the Polygon panel).

| Check | Expected |
|---|---|
| SM-G01 Layout | Title "Polygon Set Material". Name, Type, Color, Diffuse, Specular, then Smoothing, Make Default, Add to Library. OK and Cancel on the right. Defaults: Default, Default Material, 0.8 0.8 0.8, 80.0 %, 4.0 %, Smoothing on. |
| SM-G02 Colour picker | Click the swatch: the macOS colour picker opens. Picking a colour updates the three numbers live; typing numbers moves the picker. Cancel or OK closes the picker. |
| SM-G03 New material | Type "Red", set Color 1 0 0, OK. The selected polygons turn red in the Shaded view; one Undo removes both the tag and the colour. |
| SM-G04 Existing material | Select other polygons, press M, pick "Red" from the ⏷ list: the fields show Red's settings. Change Color, OK: every Red polygon changes colour. |
| SM-G05 Smoothing off | Set a material with Smoothing off on a sphere's polygons: they shade faceted, the rest stay smooth. |
| SM-G06 Keys | Return confirms, Esc cancels with nothing changed. Typing M in the Name field types M. |
| SM-G07 Save/load | Save, close, reopen the scene: tags and colours come back. Export glTF and import it: same colours. |
| SM-G08 Render | File > Render Viewport (Cycles build): the red polygons render red (darkened by Diffuse 80 %). |

## Render window (2026-11-05, branch render-polish, Cycles build)

| Check | Expected |
|---|---|
| RW-G01 Progress | Make a cube, Render > Render Viewport: a Render window opens, the picture fills in noisy and gets cleaner, "sample N of 16" counts up, then the denoised picture stays. The status line says where it was saved; Preview does not open. |
| RW-G02 Pause | Set Samples high if needed (or use a big scene). Pause (or Space in the Render window): within a second or two it says Paused and the count stops. Resume: it carries on from the same sample, and the final time leaves the pause out. |
| RW-G03 Finish | During a render, Finish: it stops at the current sample, denoises, saves, and the status says "finished early at N samples". |
| RW-G04 Cancel | During a render, Cancel, or close the Render window: it stops, the status says "Render cancelled", and no new file appears. |
| RW-G05 Region | After a render, drag a rectangle on the picture (orange outline). Move the cube a little, Render Region: only the rectangle updates (it fills in as it goes); the rest is the old picture. A new PNG is saved with the combined picture. A click on the picture clears the rectangle. |
| RW-G07 Window | The Render window is its own window, never a tab of the main one: drag it outside the main window, resize, close it with its red button. Render > Render Window shows it again. The menu bar has a Render menu (File no longer has Render Viewport). |
| RW-G08 Camera framing | View menu > Camera: a frame of the render's shape (16:9) is outlined and the outside dimmed. Render Viewport: the picture shows exactly what is inside the frame, in any viewport shape. |
| RW-G06 Size change | Draw a region, resize the main window (viewport-camera renders only), Render Region: it renders the whole picture instead and says so. |

## Render button and Preview window (2026-11-05, branch render-preview, Cycles build)

| Check | Expected |
|---|---|
| RP-G01 Buttons | Top bar, second row, right of Work Plane…: "Render" with a dimmer "F9", then "Preview" with a sphere icon, styled like the buttons beside them; tooltips "Render (F9)" and "Preview (F8)". The Render menu lists Render [F9] and Preview [F8]. |
| RP-G02 F9 | Press F9 (or click Render): the Render window opens (or comes to the front) and a final render starts. Press F9 again while it runs: no second render, the Render window just comes to the front. |
| RP-G03 Opens paused | Press F8: a window titled "Preview" opens, black, with a big light-grey Play triangle; the status bar says Paused and 00:00:00. F8 again brings it to the front and never closes it. |
| RP-G04 Progressive | Click the triangle (or the Status button): a blocky picture appears almost at once, then a sharp noisy one that cleans up; the status bar counts "Rendering – N / 256 samples", the orange strip at the bottom fills, the timer runs; at the end it says Finished and the activity dot goes dark. The main window stays responsive throughout. |
| RP-G05 Status button | While rendering, Status: Paused (timer freezes, picture stays). Again: Rendering, carrying on from the same samples. When Finished: it starts over from blocky. |
| RP-G06 Live updates | Move or edit the cube, change a material, move a light or the camera: the Preview starts over from blocky within a moment. Selecting things alone does not restart it. Options > Automatic Geometry Updates off: edits don't show until Reset Render (or pause and resume). |
| RP-G07 Resize and letterbox | Resize the Preview: the picture keeps the render frame's shape with black bars and starts over at the new size. Options > Full Resolution: renders at the render frame size, scaled to fit, and resizing doesn't restart it. |
| RP-G08 Navigate | Drag in the picture: the preview camera orbits (Shift pans, Ctrl zooms, wheel zooms) and the preview restarts; the scene's camera item doesn't move. The Pan, Orbit and Zoom icons at the top right do the same when dragged. Camera popup: pick another camera, or (Render Camera). |
| RP-G09 Image zoom | Shift+wheel over the picture zooms the image in steps (25, 50, 100, 200, 400 %); dragging then pans the image; double-click fits it again. |
| RP-G10 Lock | Click the padlock: resizing and dragging no longer restart. Click again: unlocked. Ctrl+click: locks and turns on Final Render Quality (Options shows it ticked); clicking again unlocks and turns it off. |
| RP-G11 Options | Options menu groups in order: Geometry, Quality, Light paths, Updates, Threads, Filter, Control, File, Interaction, Render, with separators. Greyed items say "Not available yet". Untick Shadows: shadows go; Global Illumination / Reflections / Refractions change the look. Show Only Selected Items renders only the selected mesh. |
| RP-G12 Effect and LUT | Effect: Alpha shows white for the object and black for the background; (Shading) and Final Colour look the same. LUT: Linear looks darker/flatter than sRGB. |
| RP-G13 Settings | The cog opens Preview Settings: Samples 256, Noise Threshold 0.050, Denoise on. Show Status Bar off hides the bottom bar; Show Viewport Controls off hides Pan/Orbit/Zoom/Maximise. The small triangle hides and shows the header rows. Quit and relaunch: the Preview window comes back at the same size and place with these settings. |
| RP-G14 F9 during Preview | While the Preview renders, press F9: the Preview pauses while the final render runs, then carries on by itself afterwards. |
| RP-G15 Close and shortcuts | With the Preview focused, app shortcuts still work (F9, Tab, W…). Close the Preview with its red button: rendering stops (CPU/GPU go quiet); reopening shows the paused triangle again. Maximise icon toggles the window's zoom. |
| RP-G16 Save Image | Options > Save Image…: a save panel; PNG or JPEG saves the picture as it looks (after the LUT). |

## Display modes (2026-11-05, branch display-modes)

| Check | Expected |
|---|---|
| DM-T1 Menu | Open the view's Display menu: exactly Wireframe, Solid, Vertex Map, Shaded, Texture, Default, Advanced, Gooch Tone Shading, Cel Shading, Reflection, in that order, with ✔ on the current one. Pick each: the ✔ moves and the button names it ("Gooch", "Cel" short). Texture: flat, unlit colours. Default looks the same as Shaded. |
| DM-T7 Advanced shadows | Cube above a plane (Make Inactive Same as Active on, or Alt-click Advanced too, so the unselected plane is Advanced): Shaded shows no shadow; Advanced shows the cube's soft-edged shadow on the plane, from the scene's light (move or rotate the Directional Light: the shadow follows; delete it: the view light's shadow). Display > Show Shadows off: the shadow goes, the rest unchanged; on again: back. Edges look smooth (anti-aliased). |
| DM-T8 New view | Launch Operandi: the view's Display button says Default. (Split View copies the current view's settings, as before.) |

## Create panel v2 (2026-11-05)

| Check | Expected |
|---|---|
| CP2-G01 Tube | Arm Tube, click four places: a tube follows all four points. Drag a point: the tube follows. Space drops it. |
| CP2-G02 Units | Ctrl-click Cone, Ellipsoid, Torus, Capsule: a unit shape lands in the current mesh (one undo each). Shift-click: in a new mesh named after the shape. |
| CP2-G03 Text | Click Text, type "Operandi" in the panel, change Font and Justification: the preview updates. Space: polygons stay, one undo removes them. |
| CP2-G04 Copy/Paste | Polygon mode, select a cube's top, Cmd+C then Cmd+V: a selected copy in place. Cmd+X removes the original. |
| CP2-G05 Paste Tool | Copy something, click Paste Tool, click three spots in the viewport: three copies on the grid. Space turns it off. |
| CP2-G06 Merge | Make two meshes, click one in the Scenes list, Cmd-click the other, Create > Merge > Merge: one mesh left, nothing moved. Unmerge splits it again. |
| CP2-G07 Ground Align | Move a cube up, Polygon mode, Ground Align, click the viewport: it sits on the grid. Ctrl-click: to the origin. Select a side face and right-click: that face turns to face down. |
| CP2-G08 Quick Align | Select one cube's polygons, Shift+R, press on another cube's face: three arrows. Release on the green one: it moves in Y to match. Right-press: two normal arrows; release on one turns the selection. |
| CP2-G09 Ruler/Protractor | Ruler: a 200 mm ruler appears; drag its ends (they snap to points), the length label follows. Protractor: drag the arms, the angle reads in degrees. |
| CP2-G10 Dimensions | Select some polygons, Dimensions: a box with X/Y/Z sizes. Orient on: the box turns with a rotated selection. |
| CP2-G11 Curve | Create > Draw > Curve. Click five spots: a smooth curve through them. Click the second point, then click empty space: the new point goes between 2 and 3. Drag a point; Ctrl-drag moves along one axis. Right-click the first point: closed. Middle-click a point: gone. Cmd+Z undoes one click at a time and the tool stays on. |
| CP2-G12 B-Spline | Same clicks: the curve starts and ends on the end points and stays inside the cage. Drop with three points: it disappears (too few). |
| CP2-G13 Bezier | Click: sharp corner. Press and drag: handles pull out both ways; one Cmd+Z removes that anchor. Drag a handle end: the other side stays in line (Keep Tangent). |
| CP2-G14 Sketch | Drag a wiggle: a curve. Change Type to Quads: the same stroke becomes a standing strip; Face: a filled outline. |
| CP2-G15 Draw Patch | Draw four Curves whose ends meet (snap the ends onto each other), then Draw Patch and click the four middle handles in order: a 10 x 10 quad surface fills them, facing you. |
| CP2-G16 Pen | Pen (Create > Draw), click three points, Shift-click: that triangle is made and a new polygon starts. Two points then Space: two loose points. |
| CP2-G17 Pen walls | Pen, Wall Mode Both Sides (Offset becomes 0.1 m), click three points: an orange strip previews along them. Right-drag right: wider. Space: quads lie in the grid plane, one Cmd+Z removes them. Inner/Outer: the strip sits on one side. |
| CP2-G18 Drag-out shapes | Click Torus, Capsule or Arc: nothing appears. Drag in the viewport: the shape draws out. Ctrl-click the cell: the unit shape lands at once. |


| SM-G09 Nothing selected | Deselect all polygons (Polygon mode), press M, type "Blue", Color 0 0 1, OK: the whole mesh turns blue, like Set Part with nothing selected. One Undo puts it back. |
| SM-G10 Button order | Polygon panel Commands: Set Part... and Set Material... are the last two rows, below Align. |

## Shader tree in the viewport (ST-V, 2026-11-06)

Viewport in Shaded, Default or Texture style; no render needed (ST-Q13).

| Check | Expected |
|---|---|
| ST-V01 Base Material | Make a box. Shading: select Base Material, set Diffuse Color red. The box turns red in the viewport straight away. Diffuse Amount 0 %: black. |
| ST-V02 Material groups | Set Material "Blue" on two polygons: those two show blue, the rest red. Eye off on the Blue group: they go red. |
| ST-V03 Constant layer | Add Layer > Constant on Base Material's level, colour yellow, 50 %: the red polygons show orange, matching a render. |
| ST-V04 Item mask | Two boxes. Group with Item set to one box, containing a Constant: only that box changes colour. |
| ST-V05 Image Map | Add an Image Map with a picture: the polygons show the picture's average colour (not the picture itself yet). Base Shader GL Display off: the average goes away. |
| ST-V06 Older file | Open a file saved before the shader tree with coloured materials: colours as before, a touch darker (Diffuse Amount now counts). |
| ST-V07 Speed | A big scene still drags and orbits as before; editing a Shading value updates the viewport without a pause. |

## Shader tree renders (ST-4, 2026-11-05)

Use Render (Cycles) after each step; the Shading tab edits what renders.

| Check | Expected |
|---|---|
| ST-R01 Base Material | Make a box. Shading: select Base Material, set Diffuse Color red. Render: the box is red. |
| ST-R02 Material groups | Set Material "Blue" (blue colour) on two polygons. Render: those two are blue, the rest red. |
| ST-R03 Constant layer | Add Layer > Constant on Base Material's level, colour yellow, 50 %: the red polygons render orange. Blend Multiply: darker. Eye off: back to red. |
| ST-R04 Group opacity | Set the Blue group's % to 50: its polygons render halfway between blue and red. |
| ST-R05 Shader | Select Base Shader, untick Visible to Camera: the box disappears from the render (shadow stays). Tick it back. |
| ST-R06 Shadow Catcher | Base Shader Alpha Type Shadow Catcher: the box renders as a shadow only (approximate on a solid background). |
| ST-R08 Image Map | Add Layer > Image Map, then in Properties Image > Load Image… and pick a picture. Render: it wraps the box by its UVs. |
| ST-R07 Older file | Open a file saved before the shader tree with coloured materials: it renders as it did. |

## Shader tree Tier 1 (branch shader-tier1, 2026-11-06)

Render (Cycles) after each step.

| Check | Expected |
|---|---|
| T1-01 Selection Set mask | Make a box, select two polygons, make selection set "Top". Add Layer > Group, Polygon Tag Type Selection Set, Polygon Tag "Top" (dropdown lists the scene's sets); add a red Constant inside it. Render: only those two polygons are red. |
| T1-02 Reflection Fresnel | Base Material: Reflection Amount 80 %. Reflection Fresnel 0 %: mirror-like everywhere. 100 %: faint straight on, strong at grazing angles. A Constant with Effect Reflection Fresnel changes it too. |
| T1-03 New defaults | New scene: Base Material shows Diffuse 0.6 grey, Specular 50 %, Specular Fresnel 100 %, Reflection Fresnel 0 %, Refractive Index 1.0, Subsurface 0.8 grey / 5 mm, Clearcoat Bump 100 %. Set Material on a new name starts at 0.6 grey / Specular 50 %. Render: Specular 50 % looks like plain plastic, not glossy. |
| T1-04 Image Map adjustments | Image Map on a picture: Swizzling Red gives a grey picture of the red channel; Alpha Ignore shows the picture's transparent parts solid; Gamma 2.2 brightens; Brightness 200 % doubles; Contrast; Clamp; Invert Green; Filtering Nearest on a small picture shows hard pixels. Effect Roughness with Low 20 % / High 60 %: black areas glossier than white but never mirror. |
| T1-05 Blend clamps | Subtract a brighter Constant: black, never negative (no odd colours with Add on top). Color Dodge / Burn stay within 0..1. |
| T1-06 Above the shader | Move a red Constant above Base Shader: no effect. Back below: red. |
| T1-08 Add Layer starts | Nothing selected: Add Layer > Constant is black at 100 % below Base Shader (box renders black); Group shows Scope All; Material has Bump/Displacement 0; Special > Shader lands at the very top, above the outputs. |
| T1-07 Older file | Open a scene saved before this branch (with Set Material colours): it renders as before; Specular shows the converted amount (old 4 % → 50 %). |

## Material brightness (branch material-brightness, 2026-11-06, Cycles build)

Render (Cycles) after each step. T1-02 above now needs Match Specular off.

| Check | Expected |
|---|---|
| MB-01 Reflection | Red material (Diffuse 0.8, 0.1, 0.1), Reflection Amount 100 %: renders as plain red plastic, not a red mirror. Turn Match Specular off: the red mirror comes back (provisional). |
| MB-02 Transparency | Same red material, Transparent Amount 100 %, Refractive Index 1.0: clear, the background shows through untinted (not red glass). Refractive Index 1.5: clear glass that bends what's behind it. 50 %: half red, half see-through. |
| MB-03 Specular | Specular Amount 100 %: only a bit shinier than 50 % (twice the reflection), much less shiny than before this branch. |
| MB-04 Diffuse | Diffuse Amount 80 % renders as bright as 100 %. Shading Model Physically Based: 80 % is darker again. |
| MB-05 Properties | Base Material: Shading Model dropdown (Principled, Physically Based, Traditional, Energy Conserving) and a Match Specular checkbox just before Reflection Color. Each change is one Cmd+Z. |
| MB-06 Save/reopen | Set Physically Based and Match Specular off, save, reopen: both come back. |
| MB-07 Older scene | Open a scene saved before this branch: Principled, Match Specular on. It looks different in the ways above (no reflection mirror, clear transparency, softer Specular 100 %, Diffuse Amount no longer darkens); that's intended. |

## Bump, normal maps, displacement (G-07, branch bump-normal-disp, 2026-11-06)

Primitives have no UVs, so import `fixtures/uv-plane.obj` (a 2 m plane with
UVs) for the image checks. Pictures: `fixtures/relief-stripes.png` (height
stripes) and `fixtures/relief-stripes-normal.png` (the same stripes as a
normal map). Render (Cycles) after each step, the plane seen at an angle
with a light grazing it.

| Check | Expected |
|---|---|
| ST-B01 Bump | Add Layer > Image Map, Effect Bump, Image relief-stripes. Render: shaded ridges across the plane, which itself stays flat (its edge is straight). Base Material Bump Amplitude 5 mm → 10 mm: ridges look twice as steep. Image Map High Value 200 % (Amplitude back to 5 mm): the same steeper look. |
| ST-B02 Bump fold | Invert on the bump layer: light and dark sides of each ridge swap. Opacity 50 %: shallower. Add a Constant above it, Effect Bump, Normal 100 %: the ridges vanish (flat). Blend Add instead: ridges come back. |
| ST-B03 Normal map | Turn the bump layers off. Add Layer > Image Map, Effect Normal, Image relief-stripes-normal: ridges like ST-B01. Same layer on a box (no UVs): the box renders plain. |
| ST-B04 Both | Bump and normal map layers both on: the ridges look stronger; drag the normal layer above / below the bump layer: the render is the same. |
| ST-B05 Displacement | Layers off. Image Map relief-stripes, Effect Displacement; Base Material Displacement Distance 20 mm. Render: the plane's edge is wavy, real ridges cast shadows. Distance 0: flat again. A Constant Displacement 50 % instead: the whole plane rises 10 mm (watch it against the grid or a box beside it). |
| ST-B06 Clamp | Two Constant Displacement layers 70 %, the upper one Blend Add: the plane rises 20 mm (clipped to 100 %), not 28. Value 200 %: also 20 mm; -100 %: sinks 20 mm. |
| ST-B07 Nothing extra | A new scene without these layers renders exactly as before, as fast as before. The viewport shows none of it and nothing crashes while editing the layers. |

## Shader tree skeleton (ST-1, 2026-11-05)

Click the Shading tab next to Scenes.

| Check | Expected |
|---|---|
| ST-G01 Default tree | Render, then Alpha Output (Alpha), Final Color Output (Final Color), Base Shader (Full Shading), Base Material ((all)), all 100 % and eye on. Below: Library, Nodes, Lights, Environments, Bake Items, FX. |
| ST-G02 Add Layer | Select nothing, Add Layer > Special > Group: it appears under Base Shader. With the group selected, Add Layer > Material: it goes inside the group. |
| ST-G03 Drag | Drag Base Material onto the group's middle: it moves inside. Drag a row near another's top or bottom edge: an orange line shows; it lands there. |
| ST-G04 Columns | Click an eye (dims the name); drag down the eyes to change several. Click B: pick Multiply, the column shows "Mul". Drag the % number left: opacity drops. Click an output's or shader's Effect: pick another. |
| ST-G05 Rename, delete | Double-click a name, type, Return. Select a row, press Backspace: it goes (the mesh stays). Undo brings each step back. |
| ST-G06 Properties | Select Base Material: Properties lists its channels. Drag Roughness: one Undo puts it back. |
| ST-G07 Set Material | Back on Scenes, Set Material "Red" on some polygons. Shading tab: a Red group with Red Material inside, Effect "Red". |
| ST-G08 Save | Save, close, reopen: the tree comes back as you left it. |
| ST-G09 Labels | Properties labels end flush against their fields in one column, for a mesh, a camera, a light and a shader layer. |
| ST-G10 Side tabs | Select a mesh: Mesh and Surface tabs down Properties' right edge; Surface shows Smoothing. Select a shader layer: one tab named for it (e.g. Render Output). The active tab has the orange edge. |

## Viewport header buttons (2026-11-06, branch viewport-buttons)

Start with View > Split View three times (four views). The buttons sit at
the right end of each view's header: Pan, Rotate, Zoom, Maximise, gear.

| Check | Expected |
|---|---|
| VB-T0 Lone view | Launch with one view: the Maximise button (double arrow) is there and orange. Click it: a quad, Top and Perspective on top, Front and Right below; your view keeps its angle in the matching slot; all four buttons white. Narrow headers drop Navigate, then Frame All / Frame Selection / Split View, instead of overlapping the menus. |
| VB-T1 Quad maximise | Set one view to Front. Click its Maximise button: only Front shows, filling the whole view area, and its button turns orange. Click again: all four views come back exactly where they were (same split sizes). |
| VB-T2 Same view | Maximise a Perspective view, orbit it and change its Display style to Wireframe, then Restore: that view keeps the new angle and Wireframe; the others are unchanged. |
| VB-T3 Keys | Pointer over a view: numpad 0 maximises it, again restores. Alt+0 does the same (laptop keyboard). Plain 0 does not. Input Map lists both as Maximise Viewport and they can be rebound. |
| VB-T3b Hover, no click | Quad view; click the Top view. Without clicking, hover Perspective and press numpad 0: Perspective maximises (not Top); again restores. Hover Front and press A: Front frames, the others don't move. Hover Right: numpad 7 makes it wireframe, numpad 1 makes it Top, . zooms it, Ctrl+Space opens the View pie and its pick changes Right. Hover Front and roll the wheel: Front zooms, the others don't. With the pointer over a side panel, A still frames the last view you used. Float the Items panel and click in it, then hover a main-window view without clicking: numpad 0 and A act on that view. |
| VB-T4 Rotate only where it orbits | Top, Front, Right (and Bottom/Back/Left) views have no Rotate button. Switch that view to Perspective: Rotate appears at once, between Pan and Zoom. |
| VB-T5 Pan | Press on Pan and drag right: the scene moves left, about 3x the pointer's travel; drag down: the scene moves up. The pointer stays hidden on the icon (keep dragging past the screen edge: the view keeps moving) and is still on the icon on release. Preferences > Input > Remapping > Reverse Pan Widget Direction on: drag right, the scene moves right. Esc during a drag puts the view back. |
| VB-T6 Zoom | Press on Zoom and drag right: zooms in; left: zooms out; up/down: nothing. Right- or middle-drag on any of the three acts the same as left; a plain click or right-click on any corner button does nothing and opens no menu. Zoom Drag Axis Bottom to Top: dragging up zooms in, sideways does nothing. |
| VBF-1 Steady drag | Press anywhere on Pan (also off-centre) and drag slowly straight right, with a little wobble: the view moves only sideways, smoothly, no jump on the first move and no creep up or down. Rotate and Zoom feel just as steady, with no sudden jumps. Release: the pointer reappears where you pressed. |
| VB-T7 Rotate rate | Drag Rotate 200 px to the right; in a second identical view, Alt-drag 200 px in the view: both end at the same angle. |
| VB-T8 Popover | Click the gear: "Viewport Properties" opens under it, tabs down its right edge. Esc closes; clicking outside closes (and selects nothing); the gear again closes. Pointer over another view, press O: it opens for that view. A long popover scrolls. |
| VB-T9 Same setting | In the popover's Visibility tab, tick Show Cameras: that view's Display menu shows Show Cameras ticked; untick it in the Display menu: the popover follows. Shading Style in Active Meshes is the Display button's style. |
| VB-T12 World scale | Two views one above the other (drag the split). In each of a Perspective and a Front view: note how big the cube is, Maximise: the cube stays exactly the same size and you see more around it; Restore: same size again. Frame All afterwards still fits. |
| VB-T10 Relaunch | Maximise a view, change a popover setting (e.g. Background Gradient), quit and relaunch: the view is still maximised and keeps the setting. |
| VB-T11 New rows | Show Viewport Controls and Information off: that view's corner buttons and top-left text go; O still opens the popover at the view's top-right. Show Meshes off: meshes vanish in that view only. Background Gradient: lighter at the top. |

Layout checks from LAYOUT-PROPERTIES-PANEL v1.2 §6. The addendum numbers
them VB-T11..T15; they are VB-T13..T17 here because VB-T11 and VB-T12 were
already taken.

| Check | Expected |
|---|---|
| VB-T13 One column | Open the gear popover in a tall and a short window, One Tab and All Tabs: one column about 420 px wide, nothing overlapping, long content scrolls; the popover never covers the view header. |
| VB-T14 Dropdowns | View Type, Background, Show Grid, Shading Style, Wireframe Overlay, Show Vertices are dropdowns (picking from one doesn't close the popover). No radio buttons, button rows or sliders; Wireframe Opacity is a "100.0 %" field with step arrows (click steps 1, drag scrubs, typing works). |
| VB-T15 Form | Labels right-aligned in one column; checkboxes stacked at the left of the control column with their names to the right. |
| VB-T16 Tabs | Full names rotated on the right edge, the chosen one orange. All Tabs shows every section in tab order; clicking a tab there scrolls to it. |
| VB-T17 Folding | Click a section bar: it folds. Close the popover and open it again (and relaunch): still folded. |
| VB-T18 Floating header | No header strip: the 3D view reaches the top of each viewport and its buttons (view type, Display, Navigate, Frame All, Frame Selection, Split View, Pan/Rotate/Zoom, Maximise, gear) float over it, each on its own dark translucent backing, white text. Over a cube that fills the view they stay readable. Clicking a button never selects or navigates; clicking between buttons does. Narrow the view: the left buttons drop out (Navigate first) and never overlap the corner buttons. The top-left info text sits just below the buttons. Show Viewport Controls and Information off hides them all. Frame All fills the whole view. |
| VB-T19 Header round 3 | The header row has only the view type and Display menus at the left and Pan/Rotate/Zoom/Maximise/gear at the right: no Navigate, Frame All, Frame Selection or Split View. Buttons are the view's background colour with a thin light edge, white text, readable over a mesh; hover and press still show. No "Viewport 2/3/4" tabs above the views: each header sits at the view's top edge. The new View menu has Frame All (A), Frame Selection (Shift+A), Split View, Maximise/Restore (or Four Views), Viewport Properties…, Tear Off View, Close View and Navigation Help (shows the mouse preset's gestures in the status line). The top-left info lines are hidden; gear > Show View Information turns them on for that view. |


## Drop Action (drop-action, 2026-11-06)

Make two boxes side by side (Item mode, no tool active). Each drop should be
one Undo step named after the action (Edit menu "Undo Parent" …).

| Check | Expected |
|---|---|
| DROP-G01 Button | Top bar: "Drop Action: (none) ⏷" right of Work Plane, left of Render. Click it: (none), Parent, Parent in Place, Match, Match Position, Match Rotation, Match Scale, Insert, Insert at Parent, Insert in Place, Place, Place and Align, the current one ticked. Choosing one doesn't add an Undo step. Quit and relaunch: the choice is kept. |
| DROP-G02 (none) | With (none), dragging one box onto the other does what it does today (nothing; a click selects). |
| DROP-G03 Parent | Choose Parent. Drag box A onto box B: A doesn't follow the pointer, B's polygon under the pointer highlights, on release A becomes B's child in the Items list (and jumps if B is moved). Undo: back as before. |
| DROP-G04 Parent in Place | Move B away, rotate and scale it. Drag A onto B: A becomes B's child without moving on screen. |
| DROP-G05 Match | Match Position / Rotation / Scale / Match: A takes B's position, rotation, size, or all three. |
| DROP-G06 Insert | Parent B under a third box C. Insert: drag A onto B: Items list shows C › A › B, B didn't move, A jumped onto B. Insert at Parent: A goes to C's position instead. Insert in Place: A doesn't move. |
| DROP-G07 Place | Place: drag a small box onto the big box's side: its centre lands on the point under the pointer. Place and Align: it also turns so its up axis points out of that face. Releasing off the big box changes nothing. |
| DROP-G08 Cancel | Start a drag, press Esc, release over B: nothing happens. Release on empty space or on A itself: nothing happens. |
| DROP-G09 Several | Select A and C, drag A onto B with Parent: both become B's children, one Undo. With Insert only A moves. |
| DROP-G10 Loop | Parent: drag B onto its own child A: status bar says it can't, nothing changes. |
| DROP-G11 Tools | With Move active, or in Polygon mode, dragging does what it did before Drop Action existed. |


## Light properties Tier 1 (light-properties, 2026-11-06)

New scene; Window/Display menu: turn Show Lights on. Each panel change
should be one Undo step named after the row (Edit menu "Undo Dissolve" …).
Render checks use F9 with the render camera.

| Check | Expected |
|---|---|
| LPROP-G01 Tabs | Select the Directional Light: Properties tabs on the right read "Directional Light", "Display", "Assembly". A mesh shows Mesh, Surface, Display, Assembly; the camera Camera, Display, Assembly. |
| LPROP-G02 Row order | Add a Spot light. Its tab: Render, Dissolve, Radiant Intensity, Light Color, Falloff Type, Shadow Type, a line, Radius, Visible to Camera, Visible to Reflection Rays, Visible to Refraction Rays, a line, Cone Angle, Soft Edge. Directional says Radiant Exitance and has no Falloff Type; Area says Radiance with Shape (Rectangle/Ellipse), Width, Height. Below: Light Material with Color, Affect Diffuse, Affect Specular, Set Temperature (Kelvins)... |
| LPROP-G03 Render | Render No: F9 render loses that light even with the eye on. Render Yes with the eye off: it still lights the render. Default: follows the eye. |
| LPROP-G04 Dissolve | Dissolve 50 %: the render is about half as bright from that light; 100 %: that light is gone. |
| LPROP-G05 Visible rows | Area light in front of the camera, Visible to Camera on: the glowing rectangle shows in the render. Put a mirror-like box (low roughness) beside it; Visible to Reflection on: the rectangle shows in the reflection. |
| LPROP-G06 Affect | Affect Specular off: no highlight from that light on a shiny box, diffuse light stays. Affect Diffuse off: only highlights remain. Each checkbox is one Undo step. |
| LPROP-G07 Temperature | Set Temperature (Kelvins)...: a small popup with a K field (6500 K) and Apply. 2000 K + Apply: Color turns orange; 10000 K: pale blue. One Undo step "Set Temperature" restores the colour. |
| LPROP-G08 Portal | Item > Create Light > Portal: an item "Portal" with a window icon in the Items list and a 1 × 1 m rectangle with an arrow in the view. Its tab: Render, Shape, Width, Height, Environment Only, nothing else and no Light Material. With Move active, its Width/Height handles work. Inside a box room with an opening and a bright world, a portal over the opening renders with less noise, not brighter. Save, reopen: the portal and its settings are back. |
| LPROP-G09 Display Visible | Display tab, Visible No on a mesh: it disappears from the view (still renders with F9) and clicks pass through it. Parent a light under it and choose No (Hide Children): the light's glyph goes too. Visible Yes on a light with Show Lights off: that light's glyph shows. |
| LPROP-G10 Label | Type "Key" in Label, Enter: "Key" appears next to the item in every 3D view; Show Label and Label Offset X/Y appear; offsets move the text. Show Label off hides it. |
| LPROP-G11 Select | Assembly > Select No on a box: in Item mode, clicking it in the view selects nothing; clicking its row in the Items list still selects it. |
| LPROP-G12 Lock | Lock Yes on a box: Item Transform fields grey out; with Move active no handles appear and dragging doesn't move it; Visible is greyed. Lock No: all back. |
| LPROP-G13 Several | Select two lights and the mesh, change Display Visible: all three change, one Undo. Select two lights, change Dissolve: both change. |

### Light properties Tier 2 (light-properties-t2, 2026-11-06)

| Check | Expected |
|---|---|
| LPROP-G14 Targeting | Add a Spot light, move it to (3, 3, 3). Targeting > Set Target: pick the mesh. The spot's cone now points at the mesh; move the mesh and the spot keeps pointing at it. Rotation fields are greyed. Roll 90°: the light turns about its aim. Enable off: back to its own rotation. Remove Target: rows collapse to Set Target. Each is one Undo step. A Point light and a Mesh Light have no Targeting section; the camera does. |
| LPROP-G15 Cylinder | Create Light > Cylinder: "Cylinder Light", a sideways tube icon, a 1 m tube of 0.2 m radius along X in the view. Rows: Render, Dissolve, Radiance, Light Color, Falloff Type, Shadow Type, Radius, Length, Visible ×3. F9 render: a long soft light along X; with Visible to Camera on, the glowing tube shows. |
| LPROP-G16 Mesh Light | Make a small box; Create Light > Mesh: rows Prototype (none) and Use Prototype Transform (on). Nothing glows until Prototype = the box; then the F9 render shows the box glowing (not grey). Use Prototype Transform off: the glowing box moves to the mesh light's position. Two mesh lights can share the box. |
| LPROP-G17 Photometric | Create Light > Photometric: Cone Angle 360°, Soft Edge 0°, Width/Height 0. Browse...: pick `fixtures/photometry/operandi-downlight.ies`: Radiant Intensity becomes 5.623, Width 0.3, Height 0.2, the file name shows, and the view draws a web stretched downward (−Z). F9: a downlight pool on a floor below it. Undo: back to no file. |
| LPROP-G18 Physical Sun | Directional light: open Physical Sun, rows greyed. Tick Physical Sun: rows enable, the light swings to the sun's direction and its rotation fields grey; the read-out says azimuth/elevation (noon, 21 June, San Francisco: about 180°/76°). Time 19:00: elevation low, light near horizontal; Time 23:00: "below the horizon" and F9 has no sunlight. Location Tokyo: Latitude 35.68, Longitude 139.69, Time Zone 9. Clamp Intensity Replace: render brightness = Radiant Exitance; None: very bright. |
| LPROP-G19 Save | Save with a targeted spot, a cylinder, a mesh light with prototype, a photometric light with the file and Physical Sun on; reopen: all of it is back. |

### Light properties Tier 3, lights (light-properties-t3, 2026-11-06)

| Check | Expected |
|---|---|
| LPROP-G20 Volumetrics | Spot light pointing down at a floor, render camera to the side. Volumetrics section (closed by default): tick Volumetrics, rows enable (Height, Base, Scattering Color, Density, Light Attenuation, Light Shift). F9: a visible cone of light in the air; Density 200 %: thicker; Attenuation 50 %: fades quickly away from the lamp. Point light: Sphere Radius instead. Area/Cylinder/Mesh/Dome have no Volumetrics. |
| LPROP-G21 LPE Label | Two lights, LPE Label "Key" on one, "Fill" on the other. F9: next to "Render N.png" in the renders folder are "Render N (Key).png" and "Render N (Fill).png", each lit by that light only. |
| LPROP-G22 Dome | Create Light > Dome: "Dome Light", a dome glyph. Rows: Render, Dissolve, Radiance, Light Color, Shadow Type only. F9 with the camera seeing sky: objects lit softly from all round, the background stays the world colour. |
| LPROP-G23 Material | Light Material: Enable off: the light gives nothing; Opacity 50 %: half as bright; Blend Mode changes nothing on its own. Add Image Layer... on a spot: pick a black-and-white picture: F9 projects it on the floor through the cone. Layer Blend / Layer Opacity / its checkbox change or remove the pattern; Remove deletes the layer; each is one Undo step. |
| LPROP-G24 Stored only | Importance, Affect Subsurface and Physical Sun Gamma edit, save and undo, with a note that Cycles doesn't use them. |
| LPROP-G25 Store Location | Physical Sun on; set Latitude/Longitude/Time Zone; Store Location...: type "Studio", Store. A "Stored Location" row appears; pick another city, then Studio from that row: the numbers come back. Quit and relaunch: Studio is still listed. |

### Shared item tabs Tier 3 (light-properties-t3, 2026-11-06)

| Check | Expected |
|---|---|
| LPROP-G26 Tabs | Any item shows Display, Assembly, User Channels and Tags tabs after its own tabs. |
| LPROP-G27 Pivot/Center | Display > Center Yes: a cross at the item's origin; Pivot Yes: a ring. Lock Yes greys both. |
| LPROP-G28 Draw Options | On a light: Add Draw Options → Enable, Wireframe, Wireframe Color appear and the button reads Remove Draw Options. Wireframe User + red: the light glyph turns red (white again when selected uses the selection colour). Remove: rows go; Undo brings them back with red. |
| LPROP-G29 Dimensions | On a box: Add Dimensions: dashed X/Y/Z edges with "X 2.000 m" etc. Visibility Selected: only while selected; Show Y off hides Y; Opacity fades it. |
| LPROP-G30 Notes | Add Notes, type a Title and a few lines: shown next to the item while it's selected. |
| LPROP-G31 Redirect / Tooltip | Assembly > Redirect = the light on a box: clicking the box in Item mode selects the light. Tooltip "Hero": hovering the box in Item mode shows "Hero" by the pointer. |
| LPROP-G32 Pick Walk | Parent two boxes under a third, Item mode, select the parent: Down selects the first child, Right the next one, Up back to the parent. Pick Walk Up = a light on a child: Up from that child selects the light. |
| LPROP-G33 Drop Action per item | Top bar (none); on box B set Drop Action: Destination = Parent. Drag box A onto B: A becomes B's child. |
| LPROP-G34 User Channels | Add User Channel...: name "Door", type Percentage, default 25, max 100: a "Door" row 25 %; drag past 100 stops at 100; a Divider adds a heading; ✕ removes. Save, reopen: kept. |
| LPROP-G35 Tags | Add Tags > Comment: a multi-line Comment row; Add Tags > custom "Pipeline" + Add Custom Tag: a Pipeline row; Remove Tags lists both. Save, reopen: kept. |

## Item groups and Light Linking (item-groups, 2026-11-06)

| Check | Expected |
|---|---|
| GRP-G01 New Group | Select two boxes, Items panel > Groups tab > New Group: a row "Group  (2)". Item > New Group from Selection makes "Group (2)". Each is one Undo step. |
| GRP-G02 Rename / Delete | Double-click a group row: type "Hero", Enter: renamed (one Undo). Delete Group: the row goes, the boxes stay. Undo brings it back. |
| GRP-G03 Members | Click a group: its items become selected. ▸ expands the member list; clicking a member selects it. Add Selected / Remove Selected change the count. Delete a member box: the group's count drops. |
| GRP-G04 Save | Save and reopen: groups and their members are back. |
| GRP-G05 Light Linking | Two boxes side by side, a group holding the left one, a point light above. Light > Light Linking (closed by default): Item Group and Mode are greyed until Control Light Linking is on. Include + the group: F9 lights only the left box (the right one is dark and gets no shadow from that light). Exclude: only the right box lit. Exclude with Item Group (none): both lit; Include with (none): neither. |
## Deform tab (deform-presets, 2026-11-06)

Make a Cylinder about 2 m tall with 8 or more height segments, Vertex mode,
nothing selected. Each tool session should be one Undo step named after the
tool (Edit menu "Undo Twist" …); each drag inside a session is a step of the
tool (Undo keeps it active).

| Check | Expected |
|---|---|
| DEF-G01 Tab | Toolbox tabs: Create, Select, Deform, Vertex, Edge, Polygon. Deform, top to bottom: Move Tools (Element Move (T), Element Push, Flex, Soft Move ▸, Smooth ▸, Soft Selection Move ▸), Rotate Tools (Twist, Bend, Vortex, Swirl), Scale Tools (Push, Sculpt, Taper, Bulge ▸), Align Tools (Radial, Linear, then Quick Align, Ground Align as on Create), Commands (Relax Mesh...). Unbuilt ones are grey, say "Not built yet" on hover, and do nothing. Each section folds. |
| DEF-G01b Flyouts | ▸ buttons have a small corner triangle. A click starts the face tool. Hold the press (or right-click): a list opens (Soft Drag, Soft Move, Shear / Smooth, Jitter, Quantize / the four Soft Selection tools / Bulge, Flare), unbuilt entries grey. Pick Shear: it starts and the face now reads Shear. Clicking elsewhere closes the list; releasing a hold doesn't start the face tool. |
| DEF-G02 Twist | Click Twist: the button lights, the Rotate handle and a Linear falloff wedge appear (full effect at the top, none at the bottom); nothing moves yet. Drag the Y ring: the top turns most, the bottom stays, the middle half as much; the cylinder doesn't get thinner in the middle. |
| DEF-G03 Falloff gives back | Set Falloff to Radial by hand, then click Taper: the falloff becomes Linear. Press Space: the falloff menu shows Radial again with your settings. |
| DEF-G04 Taper / Shear | Taper, drag a scale handle in: the top shrinks, the bottom stays (a cone). Shear, drag sideways: the top slides, the bottom stays. |
| DEF-G05 Soft Move / Swirl / Bulge | Each shows a Radial falloff sphere around the mesh. Soft Move: the middle moves most. Swirl: the middle turns most. Bulge: scaling up swells the middle while the ends stay. |
| DEF-G06 Falloff fields | With a preset on, Tool Properties lists the falloff (Start/End or Center/Size, Shape, Reverse, Fit to Selection) under the transform; editing one re-applies the drag. Dragging a falloff dot in the view also works. |
| DEF-G07 Push | Click Push, drag up anywhere in the view: every point moves out along its normal (Distance in Tool Properties); down pulls in. Polygon mode, top polygon selected, Selection Normals Selected: the cap moves up as a flat slab. |
| DEF-G08 Smooth | Select the top cap's points and Move them up unevenly, then Smooth: they even out straight away (Strength 1, Iterations 1, Lock Corner on, Sharp Angle 60°). Drag right: stronger; left: weaker. Iterations 5: smoother. Lock Boundary on an open mesh keeps the border. |
| DEF-G09 Undo | In any of them: two drags, Undo once: back one drag, tool still on. Drop, Undo once: the whole session goes. |
| DEF-G10 Refusals | Item mode: clicking a Deform button says to pick a component mode; nothing changes. |
| DEF-G11 Command line | Typing soft move, shear, twist, swirl, taper, bulge, push or smooth starts that tool. |

## Camera properties (camera-properties, 2026-11-07)

New scene, select the Camera in the Items list. Every change below is one
Undo step named after the row (Edit menu "Undo Focal Length" …); a number
drag is one step per press-to-release.

| Check | Expected |
|---|---|
| CAM-G01 Tabs | Properties tabs down the right: Camera View, Camera Effects, Display, Assembly, User Channels, Tags. |
| CAM-G02 Camera View rows | Name, Item Transform (Rotation Order ZXY on the new-scene camera), Reset, Targeting, Target Distance 4 m, Sync to View, Projection (Projection Type, Focal Length 50 mm, Lens Preset, Angle of View 39.598°, Lens Distortion), Resolution Override (Width/Height grey until the box is ticked), Images (closed), Film Back (Film Back Preset "Default (36 × 24 mm)", Film Width/Height, Offsets, Lens Squeeze, Film Roll, Film Fit Fill), Set as Render Camera. |
| CAM-G03 Lens presets | Lens Preset > 85 mm: Focal Length 85 mm and Angle of View updates. F9 render is tighter. |
| CAM-G04 Reset | Move and rotate the camera, Reset > Rotation: rotation 0, position kept. Reset > All: back to the origin. One Undo each. Assembly > Lock Yes greys Reset. |
| CAM-G05 Sync to View | In an ordinary Perspective view, orbit somewhere, press Sync to View: Camera view (View menu) now shows the same framing. Greyed while the view looks through this camera. |
| CAM-G06 Projections | Projection Type lists Perspective, Orthographic, Cylindrical, Cylindrical VR, Spherical, Spherical VR. Orthographic: Camera view goes flat and an Ortho Width row reads 2.88 m (36 mm × 4 m ÷ 50 mm). Spherical VR + Resolution Override 2048 × 1024, F9: a 360° picture with the box in the middle; Spherical: box split across the left and right edges. Cylindrical (VR): 360° around, no top/bottom. Panoramas show "Renders only" and the view stays Perspective. |
| CAM-G07 Resolution Override | Tick Override Render Resolution, 800 × 800: Camera view's frame turns square; F9 renders 800 × 800 and the Preview window is square. Untick: 1920 × 1080 again, values kept. |
| CAM-G08 Film Back Preset | Pick APS-C: Film Width 23.6, Height 15.6, one Undo. Pick (new preset), name "Mine", Save: the list now has Mine and the dropdown shows it; still there after a restart. |
| CAM-G09 Depth of Field | Camera Effects: Enable off greys Focus Distance, Autofocus, Iris rows and Edge Weighting; F-Stop stays live (4.0, minimum 1). Enable, F-Stop 1.4 (F-Stop Preset), Focus Distance 1 m: F9 render blurs the box; the viewport stays sharp. Iris Blades 6: bokeh highlights hexagonal; typing 2 gives 0 (round). |
| CAM-G10 Autofocus | Enable DOF, aim the camera at the box, Autofocus: Focus Distance becomes the distance to the box face under the centre; the frustum's end moves there. Aimed at nothing: the status bar says nothing is under the centre and nothing changes. No shortcut. |
| CAM-G11 Clipping Plane | Enable on, Clipping Distance 4.5 m: F9 cuts away the near part of the box. Distance is editable with Enable off. |
| CAM-G12 Background Image | Images > Background Image > Image > Load Image… pick a photo: Camera view shows it behind the box (behind Backdrops and the grid too); other views don't. Overscan 200 %: twice as big. Untick Background Image: hidden, image kept. F9: the photo fills the background of the render and the saved file. Set Resolution from Clip: Resolution Override on at the photo's pixel size. |
| CAM-G13 Display tab | Display shows Visible, Label, Pivot, Center, Draw Options, Notes, then Size 0.0, Show Camera Overlay, Show Stereo Volume (grey). No Add Dimensions. Size 1.0: the camera glyph becomes 1 m big and scales with zoom. |
| CAM-G14 Camera Overlay | Display > Show Camera Overlay: a Camera Overlay tab appears and opens. Camera view shows yellow action-safe (3.5 %) and title-safe (10 % / 5 %) rectangles at 50 % opacity. Grid Rule of Thirds / Center Lines / Golden Ratio / Tenths / Square each draw their lines; diagonals, chevrons, Horizon (tilts when the camera rolls). Colours and Opacity change them. Nothing shows in ordinary views or in F9 renders. Hide Camera Overlay removes the tab; Undo brings it back with its settings. |
| CAM-G15 Set Target | Select the camera, then Ctrl-click the box: Targeting > Set Target lights up; press it: the camera turns to the box and Rotation greys. Set Focus Distance on: Focus Distance greys and renders focus at the box's centre. Remove Target restores. |
| CAM-G16 Multi-select | Make a second camera, select both: F-Stop 2.8 changes both (one Undo); differing Focal Lengths show a blank field. |
| CAM-G17 Save | Save and reopen with DOF, overlay, background image and Resolution Override set: all back. A scene saved before this build still opens. |

### Camera properties: provisional picks (CAM-Q)

Build on these until Jason decides (spec camera-properties-2026-11-06;
the CP-Q1..Q7 picks are followed too).

- **CAM-Q01** "Flipped" non-VR panoramas: Spherical and Cylindrical put the
  camera's back at the picture's centre; the VR variants put the view
  direction there.
- **CAM-Q02** Cylindrical height: 360° around, and as much up and down as
  keeps pixels square (2π × height ÷ width on the unit cylinder).
- **CAM-Q03** Lens Distortion, Edge Weighting, Lens Squeeze and Film Roll
  are stored, shown and saved but don't change renders (Cycles has no
  matching control).
- **CAM-Q04** No render item yet: Film Back Preset sets only Film Width and
  Height; Set Resolution from Clip turns on this camera's Resolution
  Override at the picture's size.
- **CAM-Q05** Camera Overlay colours (safe yellow, grid light blue, guides
  light red) are ours; the other defaults are the CP-Q4 picks.
- **CAM-Q06** Autofocus measures along the view axis (Cycles focuses on a
  plane) to the first visible mesh polygon under the picture's centre (Film
  Offset moves the centre).
- **CAM-Q07** The Background Image fits inside the render frame keeping its
  shape, then Overscan scales it; it composites under final renders (F9)
  but not under the Preview window yet.
- **CAM-Q08** Set Target from the selection needs exactly the camera and
  one other item selected (either order); the Targeting dropdown still
  picks any item.
- **CAM-Q09** Rotation Order still offers only XYZ and ZXY (the six orders
  of CP-V05 need a Transform change); the new-scene camera now uses ZXY.
- **CAM-Q10** Multi-selection: numbers the cameras disagree on show blank;
  checkboxes and dropdowns show the primary camera's value.

## MCP still-life gaps (branch mcp-still-life-gaps, 2026-11-07)

Drive these from an MCP client attached with `operandi-mcp --attach`
(Window > MCP Bridge on). One pass, Jason or Codex.

- **MCP-G01** `viewport_capture {}` returns a picture of the active view
  only (no panels), and saves `Viewport NNN.png` in ~/Pictures/Operandi
  Renders.
- **MCP-G02** `viewport_capture {"view": "top"}` shows the Top view in the
  picture; the app's view flicks to Top for a frame and comes back.
- **MCP-G03** `viewport_capture {"view": "camera"}` frames what F9 renders
  (same as Camera view in the header menu).
- **MCP-G04** `view_set {"camera": <id>}` leaves the view looking through
  that camera; orbiting then moves the camera (one undo step per drag).
- **MCP-G05** `render {"width": 640, "height": 480, "samples": 32, "path":
  "/tmp/still.png"}` opens the Render window, fills in, and the call
  answers with the file once done; the window stays responsive meanwhile.
- **MCP-G06** `scene_save {"path": ".../x.opscene"}` drops the title's `*`
  and shows the file name; `scene_new` then works without a refusal.
- **MCP-G07** A light's Properties panel has a Type dropdown at the top.
  Spot → Point keeps colour, intensity and radius; Cmd+Z undoes the switch
  in one step; the viewport glyph changes with the type.
- **MCP-G08** `render_settings_set {"white_level": 0.5}` then F9: the
  picture is a stop brighter than at 1 (same as the Final Color output's
  White Level in the shader tree).

## Image Viewer, stage 1 (branch image-viewer, 2026-11-07)

Use the QA build with its own `OPERANDI_DEV_DATA` (Cycles build). Render
(F9) a scene with a camera a few times first. One pass, Jason or Codex.

- **IV-G01** F9 three times: the strip (left) shows three thumbnails,
  newest at the top, numbered 1, 2, 3; the newest has the accent outline.
  Nothing new appears in ~/Pictures/Operandi Renders.
- **IV-G02** Click slot 3: the picture shows it. Cmd+click slot 1 (also
  Ctrl+click): both outlined, slot 1 shown. Keys 1, 2, 3 (main row and
  keypad) pick the slots.
- **IV-G03** Hover a thumbnail: an x and a lock appear top right; the
  tooltip names the scene and time. Lock one: its x greys and Remove in its
  right-click menu is greyed. x on an unlocked slot removes it at once.
- **IV-G04** Right-click a thumbnail: Save Image..., Lock/Unlock, Remove,
  Show in Finder (Finder opens on the cached .exr, or the saved file).
- **IV-G05** Quit and relaunch, then Render > Open Render Window
  (Alt+Shift+F9): same strip, locks kept, newest slot shown, no render
  starts. Window place, strip/panel widths, tab and hidden panels are as
  you left them.
- **IV-G06** Preferences > Rendering > Render Window: set Max Render Slots
  to 2 with one slot locked; render twice more: the oldest unlocked render
  goes. Lock all, render again: the oldest goes anyway; the new one stays.
- **IV-G07** Turn on Also Save Every Render to Pictures, F9: a PNG appears
  in ~/Pictures/Operandi Renders and the status line says where.
- **IV-G08** Mouse wheel zooms about the pointer; Cmd+Alt-drag left/right
  zooms; the Zoom field shows %. Middle-drag pans. Turn Region off: plain
  drag pans a zoomed picture. Region on: plain drag draws the orange region
  and Render Region re-renders it. Cmd+A fits.
- **IV-G09** Point at the picture: the line under it shows X, Y and R G B A
  floats (values over 1 show as such on a bright light).
- **IV-G10** Display: Linear (raw) looks darker; Display Channels R/G/B/
  Alpha show greyscale; Show Clipping stripes the blown-out pixels.
  Saving a PNG after switching to Linear saves the darker look.
- **IV-G11** Save Image... (button, Cmd+S or the menu): pick each format
  (PNG 8/16, JPEG, TIFF 8/16, OpenEXR, Radiance HDR, Targa, BMP), Save...,
  choose a name: the file opens in Preview/another viewer.
- **IV-G12** Load Image... an EXR and a PNG: each joins the strip at the
  top and shows; they don't count towards Max Render Slots.
- **IV-G13** Header tooltips name their shortcuts (Pause shows P, Strip
  shows T, Zoom shows Cmd+A). Pass and Stereo pickers are greyed.
- **IV-G14** T hides/shows the strip; Space hides everything but the
  picture (Space again brings it back); P pauses and resumes a render;
  I / C / S bring the side panel's tabs forward (only Statistics has
  content).
- **IV-G15** Statistics while rendering: elapsed, estimated remaining,
  samples. After: total time, samples, resolution, camera, triangles,
  device, denoised. Shift+click a section arrow opens/closes all.
- **IV-G16** Quitting with slots in the strip asks nothing (IV-40).

- **LT-G01** A Point light at 3, switched to Area (1 × 1 m) and to
  Directional in the Type dropdown, lights a floor 1 m below about the same
  in F9; the Intensity field shows the converted number; one Cmd+Z
  restores type and number.
