# Missing tools, group 3: Topology tab settings and instances (v1, 2026-11-21)
Behaviour only. REA-assisted: no.

## MT7 Topology Mode (viewport draw mode)
A per-viewport on/off toggle, default off, "a drawing mode designed for re-topology".
- MT7-01 While on, active (foreground) meshes draw on top of background meshes: never hidden by a background surface they sit on or just under (a patch lying on a sphere, half buried and flickering through it with the mode off, shows whole and clean with it on).
- MT7-02 Foreground faces draw see-through at the Foreground Opacity (MT9), flat-tinted with the "Topo Face" colour (default RGB 0.2, 0.2, 0.2); foreground edges in "Topo Edge" (0.11, 0.25, 0.41), edges of back-facing polygons in "Topo Back Edge" (same default), points in "Topo Vertices" (0.38, 0.62, 0.92). Add the four colours to the colour preferences.
- MT7-03 A preference "Hide back-facing topology points" (on/off, costs speed). Default on.
- MT7-04 Background meshes draw normally (shaded) underneath. Selection, snapping and tools unchanged; display only. Saved with the viewport.
- MT7-05 The Topology tab button shows the state (lit when on). Also a command view.topologyMode and an entry in the viewport's view menu.
- Size: medium (a second draw pass for foreground meshes with depth test off and alpha).

## MT8 Offset
- Distance preference "Topology Offset", default 0.2 m, range 0 to 10 m; used only while Topology Mode is on. Shifts the foreground draw toward the camera along the view ray; geometry doesn't move (Blender-style retopology offset). Small: one depth bias in the MT7 pass. Number field on the Topology tab.

## MT9 Foreground Opacity
- Percentage preference, default 50 %, range 0–100 %. Only used while Topology Mode is on (MT7-02). Topology tab shows it as a number field bound to the preference.

## MT10 Selection Hit Size
- Second front door to the existing Selection Hit Size preference (selection_preferences, default 12 px, range 1–100). Number field on the Topology tab bound to it. Don't change its default.

## MT11 Instance Mirror (Topology tab button)
- Same Instance Mirror tool as the Duplicate tab. Replace operandi.notBuilt "Instance Mirror" with the existing command.

## MT12 Merge Instance (Topology tab button)
Bakes an instance back into its source mesh.
- MT12-01 Needs a source mesh selected that has an instance. Selecting the instance item itself, or a mesh with no instance, is refused: "Select a mesh that has an instance." No change.
- MT12-02 The instance's geometry is added to the source mesh with the instance's transform relative to the source applied (half grid x 0..2 mirrored by Instance Mirror across x = 0 becomes one mesh with both halves, the new half at x 0..−2). The instance item is deleted.
- MT12-03 Seam points are not welded (duplicate point pairs on x = 0).
- MT12-04 Afterwards the source mesh stays selected, and the added polygons are selected (Polygon mode).
- MT12-05 One undo step.
Case MI-1: grid points (i, j, 0) i, j = 0..2 (index 3j + i), quads [0,1,4,3] [1,2,5,4] [3,4,7,6] [4,5,8,7]; Instance Mirror with defaults (X axis, through origin); Merge Instance with the mesh selected -> 18 points: original 9, then 9 more at (−i, j, 0) in the same order; polygons: original 4, then [9,10,13,12] [10,11,14,13] [12,13,16,15] [13,14,17,16]; polygons 4–7 selected.
- Q-MI-1 PROVISIONAL: added polygons keep point order so mirrored half faces the wrong way. Recommendation (awaiting Jason): reverse added polygons when the transform's determinant is negative. Until then build as the reference.
- Q-MI-2 PROVISIONAL: with two instances, refuse with "Merge works on a mesh with one instance" for now.
