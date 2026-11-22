# Solid Sketch GUI check (spec solid-sketch v0.3, tests 15–26)

Use the QA app and its own data folder. Start with a new scene (one empty mesh), Perspective or Top view.

1. Create tab ▸ Draw: right-click (or hold) the Tube cell and pick Solid Sketch. Geometry ▸ Draw ▸ Solid Sketch does the same. Tool Properties shows Edit Mode, Subdiv Mode (greyed), Rotate Handles, Current Node, Center X/Y/Z, Size X/Y/Z, Twist, Auto Align, Auto Merge, Negative (greyed), in that order. Type `tool.set solid.sketch ?` once: it answers 1.
2. Click, click, click on the work plane: a chain of three 100 mm nodes joined into one smooth shape. Subdiv Mode is now available. Press and drag in empty space: the new node grows with the drag.
3. Click an old node (its dot turns highlighted), then click empty space: a branch grows from that node.
4. Drag a node: it moves and its links stretch. Shift-drag: its whole branch moves. Ctrl-click on a link: a node is inserted there. Ctrl-drag from a node: an inserted node grows with the drag.
5. Right-drag on a node: it grows (right) or shrinks (left). Shift+right-drag: the branch grows, sizes and spacing. A right-click without moving opens the viewport menu.
6. Middle-click a node: it goes and its children join its parent. Shift+middle-click: its branch goes. Set Edit Mode to Destroy: left-clicking a node deletes it; empty space does nothing.
7. On the current node: drag the red square (Size X only), the blue square (Size Z), the green square (Twist; neighbours stay). With Rotate Handles off the rings don't drag; turn it on and drag a ring (red turns about X, green about Y, blue about Z): the node turns and its branch swings. The panel follows every drag; hover a handle for its tooltip.
8. Type values into Current Node, Center, Size and Twist: the viewport follows. Type a Current Node past the last: it picks the last.
9. Cmd+Z inside the tool takes back one edit (a whole drag) and keeps the tool on; Cmd+Shift+Z puts it back. Drop (Q), then Cmd+Z once: the whole session goes.
10. Make a sketch, Drop, add a cube to the same mesh, start Solid Sketch again: the question names the cube's 6 polygons. Yes brings the nodes back and removes the cube; No starts a separate sketch. Tick Don't ask again and check the next start doesn't ask.
11. Select a mesh layer-less item (or delete all meshes) and start Solid Sketch: it refuses with "Solid Sketch needs a mesh layer. Add or select a mesh first."
12. Turn on vertex snapping with a background mesh, then start Solid Sketch on a sketched mesh: no crash.
13. Save, reopen, start Solid Sketch: Yes rebuilds the same shape. Export to OBJ: only polygons, no lines.
14. Type `operandi.solidSketch.strip` with the mesh selected: starting the tool again no longer asks.
15. Draw a three-node chain, then switch Subdiv Mode: none and Level 1 show the same polygon count (Level 1 smooth), Level 2 shows 4× as many polygons and a slightly rounder shape. Each switch updates the mesh straight away, without another click.
