# Pen Extrude — GUI check

Build: the QA app in ~/Developer/Operandi-Builds named in the thread. Use a
fresh scene. "Left-click" means the left mouse button (a plain trackpad
click); "left-drag" means hold the left button and move.

1. Create tab: left-click **Cube**, then left-click in the view to place
   it; press Space to drop the tool.
2. Press 3 for Polygon mode. Left-click the cube's top face to select it.
3. Duplicate tab: left-click **Pen Extrude**. Tool Properties shows
   "Pen Path" (no Path Steps section) and the Sweep Effector with Cap Start
   off, Cap End on, Invert on.
4. Switch the view to Front: right-click the view type button in the
   view's header and left-click Front. Left-click three points going up and
   then up-right, away from the cube. After the second click the top face
   extrudes once, straight to that point; after the third, once more.
   Expect exactly one ring of side walls per clicked point, with no extra
   rings in between.
5. The ring at the middle point leans halfway between the two segments;
   the end cap faces along the last segment.
6. Left-drag the middle point (the small square): both segments follow and
   the walls move with it. Release: one step.
7. In Tool Properties set Mode to **Delete** (left-click it), then
   left-click the last point: the last ring goes away. Set Mode back to
   **Add**.
8. Turn **Align** off: the rings stay flat (parallel to the original face)
   and only move. Turn it back on.
9. Press Space to drop. The end cap is selected. Cmd+Z once: the whole
   extrude goes away in one step. Cmd+Shift+Z brings it back.
10. Start again on another face and press Esc instead of Space: nothing
    changes.
