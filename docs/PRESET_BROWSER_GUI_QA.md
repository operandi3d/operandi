# Preset browser Stage 1: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi preset-browser <commit>.app`
(built with Cycles, so thumbnails render). Test content: the CC0 pack at
`~/Developer/Operandi-Content/Test-Library` (Poly Haven, ambientCG, EdgeCases).
Don't save into the pack unless a step says so.

1. **Open it (PB-100, PB-101).** Window ▸ Preset Browser opens it in its
   own floating window, which stays above the main window: drag it by its
   title bar (even outside the main window), resize it, close it with its
   close button. Reopen: same place and size. F6 toggles the same window
   (the first time, under the pointer); there's no fixed popover any more
   (Jason, 2026-11-11). Side panel "+" ▸ Utilities ▸ Preset Browser puts it
   in a tab. **Drag across windows (PB-110):** with the floating window
   over or beside the main window, drag a material from it onto the cube
   in a 3D view: it loads there. Click a tile in the floating window and
   use the arrow keys and Enter (PB-111).
   **Drop onto a material group (PB-110, PB1-Q02 changed 2026-11-11):**
   on a mesh with two tags that each have a group (say TableTop and
   Table_Edge), drag a material across it with the button held: only the
   polygons of the tag under the pointer highlight, the highlight switches
   when you cross to the other tag, nothing gets selected, the Shader Tree
   doesn't change. Release on the top: the TableTop group now holds the
   material's layers (its old material gone), no new group, the edge
   untouched; one Cmd+Z undoes it. Drop a second material on the top: it
   replaces the first. Esc during a drag cancels it.
2. **Add a content folder (PB-12, CD-01).** Click "(add path…)" at the
   bottom of the tree and pick `Test-Library`. It appears as a bold root.
   Open it with its arrow and click `ambientCG/Bricks113A`. Expect: one
   entry "Bricks113A", not a pile of pictures, and no .blend/.usdc/.mtlx
   files (gear ▸ "Show only usable files" is on).
3. **Thumbnails (PB-106, PB-114, CD-24).** The texture sets first show their
   own picture, then within a few seconds a rendered ball: a glossy sphere
   filling most of the picture, with a small opening up and to the right
   showing a sliver of the inside, over a soft grey gradient (no floor line,
   just a soft contact shadow; PB1-Q14, changed 2026-11-11).
   The footer's controls sit at the bottom right: size slider, then the
   Vertical, Horizontal and List buttons (the active one lit), then the gear.
   Drag the slider: the tiles resize while you drag, in both tile views; it
   hides in List. Vertical tiles show the name above the picture and the
   kind ("Material", "Texture Set"…) below in grey; Horizontal tiles put the
   picture left with the name and kind beside it. Arrow keys move one tile
   (Left/Right) or one row (Up/Down) in either tile view.
4. **Load a texture set (CD-T1, PB-T13).** Shader Tree: Add Layer ▸ Material
   Group, keep it selected. Double-click a texture set. Expect: the group now
   holds one image layer per map (colour, roughness, normal, displacement, AO
   multiply) and a material; one Cmd+Z removes it. With no material group
   selected: a message, nothing changes.
5. **Drag a material onto geometry (PB-110).** Make a cube, drag a texture set
   from the browser onto it in the viewport. Expect: the cube gets a new
   material group named after the set; one Cmd+Z undoes it. In Polygon mode
   with some polygons selected, dropping on one of them gives only those.
   Try dragging from the browser's own window too.
6. **HDRI (CD-T3).** `PolyHaven/HDRIs`: entries show as environments with a
   tone-mapped picture. Double-click one: it becomes the environment image
   (Shader Tree ▸ Environments). Another replaces it; Cmd+Z brings the old one
   back.
7. **Save into the browser (PB-T15, PB-120..122).** Select the cube. Right-click
   the Presets root ▸ Save Selection Here…, name it "Cube". It appears at once,
   and its rendered thumbnail shortly after. Save again with the same name:
   asked Replace (OK) / Keep Both / Cancel; Keep Both makes "Cube 2". Drag a
   material group from the Shader Tree onto a browser folder: a small menu
   offers Save Material Preset… and Cancel.
8. **Mesh preset (PB-T14).** Drag "Cube" into the viewport: a new item at the
   origin; one Cmd+Z removes it.
9. **Right-click an entry (PB-112, PB-T16).** Rename…, Duplicate…, Delete… (asks
   first; Cancel keeps the file), Show in Finder, Open with Default App
   (pictures), Replace Thumbnail with Last Render / Image File…, Refresh
   Thumbnail. Do these on your own Presets folder, not the test pack.
10. **Keys and search (PB-111, PB-26).** Click a thumbnail, use the arrow keys,
    Enter loads it. Type a few letters: the search opens and finds matches in
    the folder and below it. Viewport shortcuts don't fire while typing.
11. **Roots (PB-T10, CD-02, BQ-06).** Right-click a root ▸ Rename… changes only
    its label; Remove… asks, and the folder's files stay. Drag a root above
    another to reorder. Quit and relaunch: roots, view mode, size, sort and
    the window's place come back (PB-T11).
12. **Missing drive (CD-T4).** Add a folder on a USB stick, eject it: the root
    greys out with "(not found)". Plug it back in: its contents return.
13. **Big folder (PB-T12).** Point a root at a folder with ~2,000 pictures,
    scroll top to bottom twice. The second pass should not stutter.
14. **Type a command once.** `pbview.mode mode:2` (Horizontal),
    `pbview.mode mode:?` (shows "horizontal"), `pbview.mode mode:list`, then
    `layout.window hash:presetBrowser open:false`.

## Preset browser P1: matcaps and meshes (2026-11-12, branch claude/project-thread-i94glh)

Use a fresh data folder (`OPERANDI_DEV_DATA`) and add
`~/Developer/Operandi-Content/Matcaps` as a root.

1. **Fresh folders (PG-01).** Presets shows Meshes, Materials, Matcaps,
   Colours and Scenes. Meshes shows "Drag a mesh here from a 3D view or the
   Item List to save it."
2. **Matcap tiles (MC-01).** The Matcaps root's folders show round tiles,
   kind "Matcap". Hover one with a `name.txt` beside it: the note is in the
   tooltip.
3. **Use a matcap (MC-02..04).** Split the view. Double-click "Jade": the
   active view goes Matcap/Jade. Drag "Chrome" into the other view: only that
   view changes. Cmd+Z changes neither.
4. **All views (MC-06).** Right-click a matcap ▸ Use in All 3D Views.
5. **Matcap menu (MC-05).** Viewport Properties ▸ Matcap ▸ Image ▸ Library:
   the same set, one submenu per folder. Pick one.
6. **Odd pictures (MC-07/08).** Drop a 2000×1500 JPG into Presets ▸ Matcaps:
   it works (centred square). Rename the file a view uses, relaunch: that
   view shows Clay Studio and the status line says "Matcap not found: name".
7. **Save a mesh (PG-02..04).** Make a cube, move it to (2,0,0), rotate 30°.
   Drag it from the Item List onto Meshes ▸ Save Mesh Preset… (hover: the
   origin note), tick Centre on Ground, name it "Box". Its thumbnail
   appears. New scene, drag "Box" in: upright at the origin; Cmd+Z removes it.
8. **Shift-drop (PG-06).** Shift-drag "Box" onto the top of another mesh: it
   sits on that surface under the pointer. Over empty space: on the work
   plane.
9. **Model thumbnails (PG-05).** Put an OBJ in a root: within a few seconds
   its tile shows a clay render. Double-click it: added at the origin.
10. **Import Into Current (PG-07).** Right-click a scene ▸ Import Into Current
    Scene: its items join, one undo. Drag a scene into a view: imported; with
    Cmd held: opens as its own scene. File ▸ Import… adds to this scene too.
11. **Type the commands once.** `operandi.view3d.matcap name:pearl all:true`,
    `scene.importReference filename:<an .obj> lights:false`,
    `preset.saveAs kind:item filename:<file> centerOnGround:true`.
