# Modeling popup — physical-input observations, v2 draft

Modo 17.1v1, physical input by Jason with Codex observing supplied menu
captures, live viewport and exact six-decimal mesh readback. Current active
configuration; no factory reset. REA-assisted: no. These observations
supersede the earlier inability to expose a menu with synthetic input;
they do not invalidate those earlier input-delivery diagnostics.

**POP-PHY-01 — Command-left opens a popup over the selected face in Polygon mode.**
Identity unit cube, coordinates ±0.5, one face selected, Perspective view.
Physical Command-left-click inside that selected face opens the popup.
Opening/dismissing it leaves all eight points, six faces and the selected
face unchanged. Root items in order; a dash is a separator:

1. Modeling Operations (submenu)
2. Procedural Modeling Operations (submenu)
3. —
4. Set Material…
5. Shaders (submenu)
6. —
7. Add to Selection Set
8. Recall Selection Set (disabled)
9. —
10. Lasso Style (submenu)
11. —
12. Record OpenGL to Movie
13. Record OpenGL to Image Sequence
14. Record Playback (submenu)

Other named root entries appear enabled in the supplied capture.
POP-O-H1-root. No claim about other physical gestures or contexts yet.

**POP-PHY-02 — The measured Polygon Modeling Operations submenu has seventeen entries.**
Same context as POP-PHY-01. In order: Bevel, Extrude, Sketch Extrude,
Slide (disabled), Bridge, Add Loop, Smooth Shift, Shift, Inset, Chamfer
Edit; separator; Remove, Collapse, Unify, Merge, Flip, Spin, Triple.
The other entries appear enabled; none shows a further submenu arrow.
Expanding/dismissing this submenu does not change geometry or selection.
POP-O-H1-submenu. Other root submenus remain unobserved.

**POP-PHY-03 — Choosing Triple from that submenu triangulates the selected quad.**
Input selected face 1 has corners [4,5,7,6] at Z=+0.5. Physical choice
Modeling Operations → Triple removes that quad and appends triangles
[4,5,6] and [7,6,5] as faces 5 and 6. They are both selected afterward.
All eight point positions are unchanged; total faces 6 → 7. The other
five quads retain their corner order. Exact renumbering and selection:
POP-O-H1-triple. Undo/redo is not established by this case.

**POP-PHY-04 — The measured one-vertex popup differs from the Polygon popup.**
Restored identity cube, Vertex mode, only point 7 at (0.5,0.5,0.5)
selected. Physical Command-left over the cube opens the root and expanding
Modeling Operations shows, in order: Bevel, Extrude, Slide; separator;
Remove, Collapse, Split Vertex, Split Polygon, Merge, Join, Center, Align.
Join and Align appear disabled; the other nine entries appear enabled.
None shows a further submenu arrow. The root has the same ordered labels
as the selected-Polygon case except Set Material is absent; Recall
Selection Set remains disabled. The overlapping submenu capture does not
establish every root submenu indicator. Opening/dismissing changes no
geometry or selected components. POP-O-H1-vertex-menu. These enabled states
are scoped to the measured single cube corner, not every Vertex selection.

**POP-PHY-05 — Choosing Split Vertex separates the three incident faces at the selected cube corner.**
Same input as POP-PHY-04. Physical choice Modeling Operations → Split
Vertex creates two extra points at (0.5,0.5,0.5), yielding 10 points and
6 quads. The three incident quads use distinct coincident corner points
7,8,9. All original coordinates remain unchanged; faces are reordered as
recorded in POP-O-H1-vertex-split. No vertex, edge or polygon is selected
afterward. No Undo behavior is inferred from this operation.

**POP-PHY-06 — The measured one-edge popup offers the following Edge operations.**
Restored identity cube, Edge mode, only edge 5–7 selected (the vertical
edge at X=Z=0.5). Physical Command-left over the cube opens the root and
expanding Modeling Operations shows, in order: Bevel, Chamfer, Extrude,
Extend, Slide, Bridge, Add Loop, Edge Weight; separator; Spin, Remove,
Collapse, Join, Split, Grow Quads, Fill Quads, Split Normals, Edges to
Curves, Unbevel. Join and Unbevel appear disabled; the other sixteen
entries appear enabled. None shows a further submenu arrow. Root labels
and separator positions match the measured Vertex root, including the
absence of Set Material and disabled Recall Selection Set. Opening and
dismissing changes neither geometry nor selection. POP-O-H1-edge-menu.
These enabled states are scoped to this selected cube edge.

**POP-PHY-07 — Choosing Collapse merges the selected cube edge to its midpoint.**
Same input as POP-PHY-06. Physical choice Modeling Operations → Collapse
replaces the endpoints (0.5,−0.5,0.5) and (0.5,0.5,0.5) with point
(0.5,0,0.5). Counts become 7 points and 6 faces (four quads, two triangles).
The other six original point positions are retained. All component
selections are empty afterward. Complete point/face renumbering is in
POP-O-H1-edge-collapse. No Undo behavior inferred.

**POP-PHY-08 — The measured Items-mode root offers Item Operations instead of Modeling Operations.**
Restored identity cube, Items mode, only the cube selected. Physical
Command-left over it opens these root labels, in order (semicolons mark
separators): Item Operations, Properties…; Draw Style, Wireframe Color,
Fill Color; Set Work Plane; Create Item Mask, Create Item Shader, Shaders;
Add to Selection Set, Recall Selection Set; Lasso Style; Record OpenGL to
Movie, Record OpenGL to Image Sequence, Record Playback, Fusion Strips.
Recall Selection Set appears disabled; the other entries appear enabled.
Visible submenu indicators: Item Operations, Draw Style, Shaders, Lasso
Style and Fusion Strips. Opening/dismissing leaves cube geometry, item
channels and item selection unchanged. POP-O-H1-item-root.

**POP-PHY-09 — The measured Item Operations submenu has the following order and eligibility.**
Same selected-cube context. Semicolons mark separators:
Find in Item List; Delete; Duplicate…, Duplicate, Duplicate Hierarchy,
Instance, Instance Hierarchy; Change Type, Presets, Add Mesh Operation,
Merge Meshes – Selected; Parent, Parent in Place, Parent to Group Locator,
Unparent, Unparent in Place; Create Assembly Group; Select Item Hierarchy,
Pick Walk, Clear Selection Redirect; Apply to Setup/Rest, Restore Setup/Rest,
Restore Setup/Rest (Transforms Only); Select Source of Instance, Select
Instances, Select Input Items, Select Output Items.

Disabled in this case: Parent, Parent in Place, Unparent, Unparent in Place
and the last four selection entries. Other entries appear enabled. Change
Type, Presets and Pick Walk show submenu indicators; these further levels
were not opened. Expanding/dismissing changes no cube geometry, item
channels or selection. POP-O-H1-item-menu.

**POP-PHY-10 — Plain Duplicate creates another coincident Mesh item and selects only the copy.**
Same input. Physically choose Item Operations → Duplicate, the entry without
ellipsis. Mesh-item count 1 → 2. Both Mesh items have the original eight
points and six quads, position/rotation zero and scale one. The original
named HandsOnCube is unchanged; the new item is named HandsOnCube (2) in
this case, and only it is selected. POP-O-H1-item-duplicate pins both
geometries and item channels using fixture labels. No dialog was needed.
Independent editing of the two geometries is not tested here, so no broader
sharing/instance rule is inferred from their equal coordinates.
