# UV editing stage 1: GUI check

Branch `uv-stage-1`. Specs: `specifications/uv-editing-2026-11-16`. Use
the QA app from `~/Developer/Operandi-Builds` with its own data folder.
Claude can't click in Operandi, so this pass is for Jason or Codex.

Set-up: Create a Box, open View > UV View in one view (or add a UV panel
beside a 3D view), then Edit tab > UV > Unwrap... > OK.

1. **Map list.** Add the UV Maps panel (the + after the side panel's tabs >
   Data Lists > UV Maps). "Texture" shows, primary-selected. Click New Map:
   "Texture 2" appears and is selected; the UV view's map button now reads
   "UV: Texture 2" and shows no UVs.
2. **Select a map.** Click "Texture" in the list: the UV view shows it
   again. Pick "Texture 2" from the UV view's map button: the list follows.
3. **Rename.** Click "Texture 2", pause, click again (or right-click >
   Rename…), type "Iris", Enter. Cmd+Z puts the old name back.
4. **Copy and paste.** Right-click "Texture" > Copy; right-click "Iris" >
   Paste. The UV view shows the same layout in both maps.
5. **Delete several.** New Map twice more. Ctrl-click (Cmd-click) both new
   maps, right-click one > Delete 2 Maps. One Cmd+Z brings both back.
6. **Unwrap into the selected map.** Select "Iris", Unwrap... > OK:
   "Texture" doesn't change (check by selecting it).
7. **Move in UV.** Hover the UV view, press W. Flat handles appear at the
   selection's centre: red U arrow, green V arrow, yellow square. Drag the
   red arrow: the UVs slide sideways only; the 3D box doesn't move. Drop
   (Space). Cmd+Z undoes the move in one step.
8. **Move with nothing selected / with a selection.** Clear the selection
   and Move: every UV moves. Select one polygon in Polygon mode and Move:
   it and the UVs it shares with neighbours move; UVs across a seam (pale
   lavender edges) stay.
9. **Ctrl.** With Move, Ctrl-drag off the handles: the move locks to U or
   V. Press E (Rotate) in the UV view and Ctrl-drag round the ring: the
   angle goes in 15° steps; dragging counter-clockwise is a positive angle.
10. **Scale.** Press R in the UV view. Drag the yellow centre square: both
    axes scale; drag the red box: only U. Tool Properties shows Scale U /
    Scale V.
11. **Tear Off.** Select one polygon, Move, Ctrl+Shift-drag (Cmd+Shift):
    the polygon's UVs come away from their neighbours. Transform (Y) also
    has a Tear Off check box in Tool Properties.
12. **Snapping.** Turn Snapping on and Move in the UV view: the selection's
    centre lands on the 0.1 grid lines.
13. **Fit UVs.** Edit tab > UV > Fit UVs: the UVs fill the 0-1 tile, keeping
    their shape. Shift-click: each island fills the tile on its own. Alt-
    click: each polygon fills the tile. Cmd-click (Ctrl-click): the Fit UVs
    dialog opens with the last values (Fit, Keep Proportion, Gaps, Gaps by
    Pixel, UDIM); set Gaps 10 % and OK.
14. **Flip and Mirror.** Flip U mirrors left-right about the selection's
    centre; Alt-click flips each polygon on its own. Mirror U mirrors about
    U = 0.5; Cmd-click opens its dialog to change Center.
15. **Align.** In Vertex mode, select a few points of a row in the UV view
    and click Align Down: they all move to the lowest V. In Edge mode,
    select two separate edge rows and Alt-click Align Up: each row
    straightens on its own.
16. **Paint select.** In the UV view, drag across polygons without a tool:
    each one passed over is selected (Shift adds, Ctrl removes). The 3D
    view shows the same selection.
17. **Lasso.** Right-drag a loop round some UVs in the UV view: what lies
    inside is selected (Shift adds, Ctrl removes).
18. **Checker in UV.** Click "Checker" in the UV view's header: a grey and
    white check numbered 1-64 sits behind the UVs, brightest in the 0-1
    tile.
19. **Checker in 3D.** In a 3D view open the shading menu (the view's
    Display button) and tick Checker: the box shows the numbered check
    through the selected map. Scale some UVs in the UV view: the squares on
    the model get bigger or smaller with them; a stretched UV shows
    stretched squares. Untick it: the box looks as before. New views start
    with it off.
20. **Save and reopen.** With "Iris" selected and some UVs moved, save,
    close and reopen: the same map is selected and the UVs are where you
    left them.

## Getting to UV editing (UVT-200..203)

21. **Layout bar.** Under the menu bar there are two tabs, Model and UV.
    Click UV: the toolbox opens at its UV tab (Map field at the top, Move /
    Rotate / Scale / Transform, Unwrap, Pack, Fit, Flip, Mirror, Align),
    the view area shows a UV view on the left and a Perspective view on
    the right, half each, and UV Maps is among the right-hand tabs.
    Selecting in either view shows in the other.
22. **Map field.** Pick a map in the UV tab's Map field: the UV view's map
    button and the UV Maps list follow, and the other way round.
23. **Back to Model.** Click Model: your own views and toolbox tab come
    back and the UV Maps tab added by the UV layout goes. Click UV again:
    it's as you left it.
24. **Reset.** In the UV layout, split a view, then double-click the UV tab:
    a "Reset Layout" question appears; Cancel changes nothing, Reset puts
    the two-view arrangement back.
25. **UV Editor.** In the Model layout choose Texture > Open UV Editor: a
    UV view opens in its own window, about 800 × 600, with the UV tools
    down its right side. Move UVs there (W, drag) and watch the 3D view.
    Choosing it again doesn't open a second window.
26. **Double-click a map.** Close the UV Editor, then double-click a map in
    the UV Maps list: it becomes the selected map and the UV Editor opens
    on it. The name doesn't turn into a rename field.
27. **View > UV View** still switches the active view between 3D and UV.

## Transform pipe on UVs (UVT-130..137)

Use a cube with its UV map (Cube tool, Make UVs on), UV layout, Polygon mode.

28. **Move a UV polygon.** Select one polygon in the UV view, W, drag the
    teal U arrow: only that polygon's UVs (and UV points it shares inside
    its island) move; across a seam the neighbour stays. The 3D view
    doesn't change shape. Drop, Cmd+Z: one step back.
29. **Rotate / Scale.** E, drag the ring: the polygon turns counter-
    clockwise when you drag counter-clockwise. R, drag the corner square:
    it scales in U/V. Handles are flat: U teal, V yellow.
30. **Element Move a point.** Vertex mode, T (Element Move), Range 0. Hover
    over a UV point: it lights up. Drag it: only that UV point moves; on a
    seam, only the side you grabbed.
31. **Element Move an island.** Set Use Connectivity, right-drag sideways
    to grow Range (a circle shows the size) until it reaches the next
    island, then drag a point: the other island doesn't move. Rigid
    Connections: the whole island moves.
32. **Soft Move / Taper.** Start Soft Move (Deform tab) in the UV view with
    a few polygons selected: the falloff ellipse is drawn around the UV
    selection; dragging moves the middle most, the edge not at all. Taper:
    the falloff line is drawn in UV, one end scales, the other stays.
33. **Smooth.** With UVs pulled out of shape, start Smooth in the UV view
    and drag right: the UVs relax, the 3D mesh doesn't.
34. **Slide.** Edge mode, select an edge in the UV view, start Slide there
    and drag sideways: the UV edge slides along its UV neighbours.
35. **3D unchanged.** In the Perspective view, the same tools (W, T, Soft
    Move) still move the mesh and leave its UVs alone.
