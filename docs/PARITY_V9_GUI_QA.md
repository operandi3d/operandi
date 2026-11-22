# Spec v9 batch: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi parity-v9 <commit>.app`. Left/right/middle refer to mouse buttons.

1. **Axis Rotate:** Cube, Polygon mode, select all (Cmd+A). Tools menu ▸ Axis Rotate (or Alt+Shift+E): no ring yet. Left-click on the cube's front face: one ring appears there. Drag the ring: the cube turns about that spot. Type Angle 30 and click Apply: it turns 30°.
2. **Soft Drag Transparent:** Deform tab ▸ Soft Drag on a segmented cube from the front, left-drag over the middle: only the front face moves. Tick Transparent (Screen Falloff, Tool Properties) and drag again: back points under the circle move too.
3. **Topology Pen:** turn on Constrain to Background, then (setup as in TOPOLOGY_TOOLS_GUI_QA.md) left-drag a point of the square: it moves without sticking to the sphere. Shift+left-drag an edge: new strip on the sphere. Shift+middle-click an edge: a loop is added across it. Ctrl+middle-click an inner edge: it's removed; on a border edge the status says no.
4. **Tack:** tick Copy Geometry, left-click a sphere polygon: a copy of the cube lands there, the original stays. Untick it and Shift+left-click: same, once. Tick Make Bridge and click: nothing happens (status explains).
5. **Retopo Edge Lock / Flow:** Polygon mode, select a curve drawn with Retopo Sketch; Edit tab ▸ Commands ▸ Retopo Edge Lock, then Retopo Edge Flow: the curve is in both sets (Select tab ▸ Use Selection Set lists RetopoEdgeLock and RetopoFlow).
6. **Dicer:** Edit tab ▸ Axis Slice: one flyout button (Polygon Axis Slice on its face). Right-click (or press and hold) it: Polygon Slice, Polygon Axis Slice, Pen Slice, Primitive Slice, Dicer. There's no Julienne button any more.
