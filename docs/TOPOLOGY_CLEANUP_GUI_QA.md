# Topology and clean-up: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi deform-finish <commit>.app`.

1. **Triple Fan**: make a Cylinder with 8 sides; Vertex mode, click one corner of the top cap; Polygon mode, click the cap; Polygon tab ▸ Triple Fan: the cap becomes 6 triangles all meeting at that corner. Nothing is selected after; Cmd+Z puts the cap back.
2. **Triple Fan, wrong selection**: with no vertex selected, Triple Fan says what's missing and changes nothing.
3. **Collapse Selection**: a Cube with Segments 4; select 2 x 2 polygons on one face; Collapse Selection: they become 8 triangles around one point in the middle; the face's outline doesn't move.
4. **Axis Rotate**: Tools ▸ Axis Rotate (or Alt+Shift+E) in the Top view: only the Y ring shows; dragging it turns the selection about Y. In the Front view it is the Z ring.
