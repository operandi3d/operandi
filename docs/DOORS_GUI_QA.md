# Phase 0 doors: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi project-thread-bjattn <commit>.app`.
Fixture: `fixtures/kit-item-splat.opscene` (in the repo).

Nothing here should look different in an ordinary scene: steps 5-7 check
that everyday selecting still works the same.

1. **Kit item opens.** File > Open `fixtures/kit-item-splat.opscene`. It
   opens with no error. The Items list shows "Garden splat (kit missing)"
   with a red "?" after its name; hovering the row says it needs kit
   "splat" and is kept unchanged.
2. **It draws.** A dashed box (2 × 1.5 × 1 m) sits right of the origin.
   Click the row in the Items list: the box turns the selection colour.
3. **Saves back unchanged.** File > Save As… a copy, close it, open the
   copy: the item, its "?" and its box are back.
4. **Scripts.** In the command entry type
   `item.channel name:count value:? item:1` → answers 120000.
   `item.channel name:count value:5 item:1` → refused, "Needs kit…".
   `item.channel name:fps value:? scene:true` → 29.97 (stored exactly as 30000/1001); `item.channel name:frame value:? scene:true` → 24.
   `item.channel name:fps value:25 scene:true`, then Cmd+Z: back to 29.97
   (one undo step, "Scene Time" in the Command History).
5. **Click select.** New scene, add a cube, Polygon mode. Click a
   polygon: selected. Shift+click another: both. Ctrl+click one: removed.
   Click empty space: selection cleared.
6. **Paint and lasso.** Drag-paint across polygons, then a lasso: both
   select as before.
7. **Mesh Stacks still show.** Add a Mesh Stack (Mesh Stack panel, add a
   Cube op) to an empty mesh:    Render shows it; File > Export OBJ contains it.
