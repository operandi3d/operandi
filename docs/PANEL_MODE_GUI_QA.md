# Panel mode: GUI check (UIC-100..124)

For Jason or Codex. QA build from branch `panel-mode`. Use the QA app's
own data folder.

1. Select a mesh. Properties: the first row on every tab (Mesh, Surface,
   Display, Assembly, User Channels, Tags) is **Name** (UIC-104).
2. Mesh tab: rows are label | small circle | field. Labels end on one
   edge, every field starts at the same x and ends 6 px from the scroll
   bar. "Position X", then "Y", "Z" (UIC-114). Move the item: Position's
   circles fill ("Constant"); set it back to 0: they empty (UIC-112).
   Clicking or right-clicking a circle does nothing.
3. Drag the small arrows at the right of Position X sideways: the item
   moves live (Shift coarser, Option or physical Ctrl finer). Cmd+Z once
   puts it back (one drag = one step).
4. Click the darker "Item Transform" bar anywhere: it folds. Quit and
   relaunch: still folded (UIC-105..107). Unfold it.
5. Activate Bevel (polygon) and Extrude: every number field in Tool
   Properties has the arrows at its right end, value read from the left.
   Checkboxes have no left label; the box sits where fields start.
6. Edge Bevel: Sharp, Mitering, Coplanar… still show greyed (Jason's
   call). Tweak: Always Raycast / Mix Mode / Slip UVs are gone (UIC-117).
7. Preferences (Cmd+,): each page is darker section bars and rows. UI page:
   Popup Transparency is a number field with "%" and arrows, no slider;
   change it: its circle fills. Grid page: Grid Type is a dropdown, not
   radio buttons. Tool Handles: every field has arrows.
8. Preferences > File I/O: Default Image Format is a dropdown; PNG
   Compression Level and JPEG Quality are number fields.
9. Snapping Options (F11 or the menu): Mode dropdown, Grid and Vertex only,
   Ranges section with px fields and Grid Distance.
10. Preview > Preview Settings, Work Plane Settings, Smooth Shift's Mini
    Properties: same rows; Work Plane's Plane is a dropdown.
11. Hover any row's label or field: it has a tooltip (UIC-119). A long
    label cut with "…" shows the full name in its tooltip (UIC-111).
12. Viewport Properties (O over a view): section bars look like the
    Properties panel's; Wireframe Opacity drags from its arrows.
13. Properties tab strip: the active tab's name is orange, the others grey;
    no tab is filled or pushed out (UIC-100).
14. Select two meshes at different positions (Shift+click in Items):
    Position X reads "(mixed)", its circle filled. Type 2 and Return: both
    move to X 2 m; Cmd+Z once puts both back. Drag Position X's arrows:
    both move, keeping their gap (UIC-56/57/118).
15. Select two point lights, change one's intensity: Radiant Intensity
    reads "(mixed)"; set it: both take it, one undo step. Assembly tab with
    two items whose Select or Tooltip differ: "(mixed)" until set.
