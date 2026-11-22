# Node View panel: GUI checklist

Branches `node-view-dock` and `node-view-close`. Use the QA app; Model layout, one box. Make a
Material Group (Shader Tree: Add Layer > Group, drag Base Material into it).

1. **Opens docked.** Right-click the group > Show as Nodes. No separate
   window opens: a "Node View" tab docks under the 3D view(s), across their
   full width, taking about 40 % of their height. Boxes, wires and the
   values column on its right work as before (drag pans, scroll zooms,
   right-click adds nodes).
2. **Only one.** Show as Nodes again, or Window > Panels > Node View: the
   same panel comes to the front; no second copy. Window > Panels shows
   Node View ticked.
3. **Follows the selection.** Add a second Material Group. Click a layer
   inside it in the Shader Tree: the Node View switches to that material.
   Click the first group: it switches back. Click Render or empty space:
   the view keeps the last material (it doesn't go blank).
4. **Explicit open wins.** With a layer of group 2 selected, right-click
   group 1 > Show as Nodes: the view shows group 1 until you click
   something else in the tree.
5. **Graph layers.** Import a MaterialX file (File > Import MaterialX…,
   `fixtures/materialx-examples/StandardSurface/standard_surface_marble_solid.mtlx`)
   and click its layer: the Node View shows its graph, read-only.
6. **Docks like any panel.** Layout > UI Build Mode (Ctrl+Shift+L). Drag the
   Node View tab: the dock cross appears; dock it on the right of the view,
   then as a tab beside Items, then drop it in empty space so it floats in
   its own window. Done. Each place works; the canvas still pans and zooms.
7. **Saved with the layout.** Dock it somewhere unusual, quit, relaunch: it
   is where you left it. Layout > Reset Layout: it's gone (the shipped
   layout has none); Show as Nodes docks it under the views again.
8. **Closing (branch `node-view-close`).** Each of these closes it, and
   Window > Node View (Cmd+Shift+N) brings it back showing the same
   material, pan and zoom:
   - the **×** at the right end of its header (hover says "Close Node
     View" with the key);
   - right-click its header > **Close Panel**;
   - **Window > Node View** or **Cmd+Shift+N** while it's open and in
     front: the menu entry (and Window > Panels > Node View) is ticked
     while it's open; pressing again closes it, again reopens it;
   - floating in its own window (step 6): the window's red close button
     closes the panel (it doesn't jump back into the main window).
8a. **Full view.** Pointer over the Node View, press Numpad 0 (or Alt+0):
   it fills its group. The × is still in its header. Esc with the pointer
   over it puts the layout back; so does Numpad 0 again. Maximise again,
   then Cmd+Shift+N: first press puts the layout back, second closes it.
9. **Keys over it.** Over the canvas, Delete deletes the selected node (not
   the mesh in the 3D view); over the 3D view, Delete works on the mesh as
   usual. Cmd+Z works with the pointer over either.
10. **Empty.** In a fresh scene, Window > Panels > Node View: it opens
    saying "Select a material in the Shader Tree to see its nodes".
