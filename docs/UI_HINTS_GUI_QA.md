# UI hints: GUI check

For Jason or Codex. QA build from branch `ui-hints-1`.

## Greyed controls explain themselves

1. Polygon mode, Select tab: start Move, then hover a greyed Selection
   button (Grow, Loop): the tooltip says "Drop or Cancel the active tool
   first". Drop the tool; switch to Edges mode; hover Boundary: "Switch to
   Polygons mode first".
2. Vertex tab in Polygon mode: hover a greyed row: "Switch to Vertices mode
   first".
3. Item Properties of a locked item (Assembly > Lock): hover the greyed
   transform rows: "The item is locked (Assembly > Lock)". Hovering a
   greyed group of rows anywhere in it shows the reason (it didn't before).
4. Camera properties: untick Depth of Field's Enable, hover Focus Distance:
   "Turn on Depth of Field first".
5. A dialog with a dependent row (e.g. Subdivide Options): hover a greyed
   row: it names the row to change ("Set Method to … first").
6. Cmd-click the viewport in Edge mode: hover a greyed Polygon entry:
   "Switch to Polygons mode first" (it said "Not available for this
   selection").
7. Open a pie menu with a greyed slot (e.g. View pie, Camera with no
   camera) and point at it: the reason shows under the slot.
8. macOS menu bar: with a tool active, open Select and hover a greyed entry:
   the macOS tooltip gives the reason. (Check whether macOS shows tooltips
   on greyed menu items at all; note it if not.)

## Active-tool exit hint

9. Start Move: the viewport's bottom-left line ends "Space / Q: drop tool ·
   Cancel: Tool Properties".
10. Input Map: rebind Drop to another key, Save. The line shows the new key
    straight away. Clear Drop's binding: only "Space: drop tool" is left.
11. Cube tool: the line shows "Space / Q / Escape: drop tool". Create tool
    (Vertex tab): it ends "Escape: cancel". Smooth Shift: no "Space" (Space
    toggles its mini properties there).
12. Drop the tool: the line goes away.
