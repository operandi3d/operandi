# Soft Drag, Element Push, Soft Selection: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi deform-hands-on <commit>.app`. Start with a new scene: Create tab ▸ Cube, then set Segments X/Y/Z to 10 in Tool Properties, press Space to drop it. Switch to the **Deform** tab. "Left-drag" means hold the left mouse button and move; "right-drag" means hold the right mouse button and move.

## Soft Drag (Move Tools ▸ the Soft Drag button; it is the face of its flyout)

1. Vertex mode (press 1), nothing selected. Left-click the **Soft Drag** button.
2. Left-drag on the cube's surface: the points under a small circle (20 px) move with the pointer, fully at the centre, fading to nothing at the circle's edge.
3. Left-drag somewhere else on the cube: the circle jumps to where you pressed.
4. Right-drag in the view, moving the mouse to the right: the circle grows; to the left: it shrinks (the status line shows its size in px).
5. Select a few points (left-click, Shift+left-click), then left-drag over them: only selected points move, still faded by the circle.
6. Points on the far side of the cube don't move (Transparent is off).
7. Press Space to drop; Cmd+Z undoes the whole Soft Drag.

## Element Push (Move Tools ▸ Element Push)

8. Any mode, nothing selected. Left-click **Element Push**, then move the pointer over the cube: the vertex, edge or polygon under it highlights.
9. Polygon mode (press 3): left-drag up on a polygon: it moves out along its normal as one flat piece; left-drag down pulls it in.
10. Vertex mode: left-drag a point: it moves along its normal.
11. In Tool Properties set Element falloff **Range** to 0.3: left-drag a polygon again: its neighbours follow it in the same direction, less the further they are.

## Soft Selection (Move Tools ▸ the Soft Selection Move flyout; right-click or press and hold it for Rotate / Scale / Transform)

12. Vertex mode, select one point on a face. Left-click **Soft Selection Move**: the handle's axes line up with the selection (its blue Z arrow points along the face's normal).
13. In Tool Properties set **Radius** to 0.5. Left-drag the handle's Z arrow: the point moves fully; points within 0.5 follow less and less; points further away stay.
14. Drop the tool (Space). With the same mesh active, Create tab ▸ Cube and left-drag a second small cube right beside the first, almost touching: it goes into the same mesh but isn't connected. Back on the Deform tab, select a point near the gap, Soft Selection Move, Radius 0.5. With **Use Connectivity** on, dragging leaves the second cube still. Turn it off: points of the second cube inside the Radius move too.
15. Right-click the flyout and pick **Soft Selection Rotate**: dragging a ring turns the selection, fading the same way. Scale and Transform the same.
16. Press Space to drop; the falloff you had before comes back.
