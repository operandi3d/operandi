# Tack, Topology Pen, Retopo Sketch: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi topology-tools <commit>.app`. "Left-click" / "left-drag" use the left mouse button; "Shift+left-drag" means hold Shift, then left-drag.

Setup (used by all three): Create tab ▸ **Sphere**, left-drag a large one (about 2 m) in the view: this is the background. In the Items list, click **New Mesh** (or press N) so a second, empty mesh is active and the sphere is now a background mesh.

## Tack Tool (Tools menu ▸ Tack Tool)

1. With the new mesh active, Create tab ▸ **Cube**, left-drag a small cube (about 0.2 m) beside the sphere. Press Space.
2. Polygon mode (press 3), left-click the cube's bottom face to select it.
3. Tools menu ▸ **Tack Tool**. Left-click anywhere on a polygon of the sphere facing you: the cube jumps so its bottom face sits flat on that polygon's centre, standing out along the sphere's surface.
4. Orbit round and left-click a polygon you're seeing from behind: nothing happens, the status line says it's the back.
5. In Tool Properties set **Offset** to 0.1 and click again: the cube sits 0.1 m off the surface. Turn **Align** off: the cube moves without turning.
6. Cmd+Z undoes each click on its own.

## Topology Pen (Edit tab ▸ Topology Pen, or Tools menu)

7. With the new mesh active, Create tab ▸ **Cube**, make a small cube near the sphere's surface, then in Polygon mode delete all but one face (select the others, Delete) to leave one square near the sphere.
8. Click **Topology Pen**. Left-drag an edge of the square: it moves and does **not** snap to the sphere.
9. **Shift+left-drag** an edge outward and release: a new square grows from that edge, and its far edge lies on the sphere's surface.
10. Cmd+Z removes the new square in one step. Space drops the tool.

## Retopo Sketch (Edit tab ▸ Retopo Sketch, or Tools menu)

11. With the new mesh active, click **Retopo Sketch**. Left-drag across the sphere from one side to the other and release: a curve appears on the sphere's surface, following your stroke (status: "a curve of N points"; N about one per 3 cm).
12. Left-drag in empty space off the sphere: no curve; the status says to drag across a background mesh.
13. Cmd+Z removes the curve.

## Topology Pen v2 modes (branch topology-pen, 2026-11-20)

Same scene: a single square near the sphere, then a few more squares made with Shift+drag. Topology Pen up, Vertex mode.

14. Tool Properties ▸ **Mode** lists Move, Duplicate, Remove, Split, Add Loop, Point, Fill, Smoothing. Values for the other modes are greyed.
15. **Remove**: click an inner edge and the two squares join. Click a polygon and it goes. Click a border edge: nothing happens, and the status says border edges can't be removed.
16. **Split**: drag from one corner of a square to the opposite corner and release: two triangles. Release on a neighbouring corner: nothing, and the status says why.
17. **Add Loop**: press on an edge, slide along it and release: a loop at that spot. Turn on **Split at the Middle** and it always cuts at 50 %.
18. **Point**: click empty space over the sphere and a dot appears on the surface. Make four in a square. **Fill**: hover between them to see the outline, then click: a quad facing you, and the dots are gone.
19. **Smoothing**: drag over the patch and the points relax but stay on the sphere. Turn on **Lock Boundary** and the border stays put.
20. Back in **Move**: right-drag an edge to move its whole loop. Right-drag empty space to change Radius. Middle-drag to split. Ctrl+click empty space for a new square facing you. Ctrl+drag a point to slide it along its edges. Shift+Ctrl+drag to smooth.
21. Drag a border point onto another border point: it jumps there, with a ring. Release and the two points weld in one undo step.
22. Topology tab ▸ **Topology Pen**: snapping turns off while it's up. Space drops it, and snapping is back as it was.
