# G8 Replica family (10 buttons, one new item)
REA-assisted: no. Sources: the reference's plain-text tool preset list and option names, its public user guide, and scripted tests (2026-11-21). No internal names, screenshots or vendor files carried over.

## The short version
Each Replica button is an existing Operandi duplicate tool with a different ending. The placement part is the same tool Operandi already has as the "Instance ..." version. Instead of N instance items, a Replica tool makes one Replicator: a hidden point cloud with one point per copy; a link to the original item (the "prototype"); a group that holds both. So: build the Replicator item once (RP-1..RP-5), then give the ten buttons a "make a Replicator" output (RP-6).

Button (Duplicate tab) = placement tool: Replica Clone = Clone / Instance Clone; Curve Replica = Curve Clone (curve path, Steps); Bezier Replica = Bezier Clone; B-Spline Replica = B-Spline Clone; Pen Replicate = Pen Clone (corner option off); Replica Scatter = Scatter Clone; Replica Array = Array; Replica Radial Array = Radial Array; Replica Mirror = Mirror; Mesh Paint Replica = Mesh Paint (RP-7). Each sits in the same drop-down or flyout as its Instance sibling, with the label shown.

## RP-1 What one Replica tool makes (observed)
Select a mesh item "proto", run Replica Array with 3 x 1 x 2 copies and X offset 2 m. Three new items:
- "proto Replicator": a group (locator-style item) at the root.
- "proto (Point Cloud)": a mesh with points only, no polygons, child of the group, hidden (visibility off).
- "proto (Replicator)": the Replicator item, also a child of the group. Point source = the point cloud, prototype = "proto".
- The original stays where it is, visible, and counts as the first copy. The point cloud has a point only for the other copies: 3 x 2 = 6 places gives 5 points, with (0,0,0) left out. Radial Array with 4 copies gave 3 points. Mirror gave 1 point.
- Each point is where that copy's pivot goes. For Radial Array around the origin all three points were at (0,0,0), but the copies showed at 90°, 180° and 270°. So each point also carries that copy's rotation (and scale, and the mirror flip for Mirror). How Operandi stores that per point is up to the builder.
- Selection afterwards: the original stays selected. Undo: one step removes all three items.

## RP-2 Replicator settings (shown in Properties)
Point Source = the point cloud (any mesh can be chosen; its points, or in Surface mode its polygons, are used). Prototype = the original item (item or group; with a group each point picks one member at random by Seed). Use Source Transforms = off (on: each copy also takes the prototype's own position/rotation/scale). Hierarchy = off (on: copies include the prototype's children). Random Position X/Y/Z = 0 m. Random Rotation X/Y/Z = 0°. Random Scale X/Y/Z = 0 %. Uniform Scale = on (one random scale for all axes). Base Scale = 100 %. Snap Rotation = off (random rotation snaps to whole steps; step PROVISIONAL RP-Q1). Density = 100 % (share of points that get a copy in renders). Display Density = 100 % (share drawn in viewport). Seed = a number (401 seen), drives every random choice so results repeat.
Randoms apply on top of each point's own transform. Model on Blender's Instance on Points + Random Value: uniform value in [−range, +range], or [−range, +range] % for scale.

## RP-3 Viewport drawing
- By default every copy draws as a wireframe bounding box of the prototype. The original draws normally.
- Per-viewport option "Replicators: None / Selected / All": None (default) = boxes everywhere; Selected = shade the copies of the selected Replicator; All = shade every Replicator's copies. In the viewport options with the other display toggles.

## RP-4 Rendering
Copies render as full geometry with the prototype's materials, via instancing (Cycles instances; USD point instancer on export). Density (not Display Density) decides how many render.

## RP-5 Freeze Replicator
Item menu command Freeze Replicator: turns a Replicator into a group of ordinary instance items, one per copy, at the same places. The Replicator and its point cloud are removed. PROVISIONAL (RP-Q2).

## RP-6 The ten tools
- Each tool's panel = the placement tool's panel plus a Replica section: Source = Active Item (default) / Specific Item / All Background Meshes / Mesh Layer Preset / Random Presets (what gets replicated); Parent = Scene (default) / Selected (where the new group goes).
- Placement defaults = the Instance version's. Replica Mirror with defaults made 1 point. Replica Clone made 1 point at the original's place (Clone's default offset is zero). Replica Scatter made a random cloud inside the item's bounds.
- Dropping the tool with nothing to copy does nothing and shows "Select an item to replicate".
- Running a Replica tool again on the same item makes a second Replicator (names get a number suffix).

## RP-7 Mesh Paint Replica
Mesh Paint (G6) with the Replica output, Source defaults to All Background Meshes. Each dab adds points to the one point cloud made when the stroke starts; one stroke = one Replicator. Prototype = the background mesh(es); with several, each dab picks one at random.

## Open: RP-Q1 Snap Rotation step; RP-Q2 Freeze naming; RP-Q3 Surface mode (build Vertices first).

## Tests to add
1. Replica Array 3 x 1 x 2, X offset 2 m: three new items, point cloud hidden with 5 points [(2,0,0), (4,0,0), (0,0,1), (2,0,1), (4,0,1)], original unchanged.
2. Replica Radial Array, 4 copies: 3 points; copies drawn at 90°, 180°, 270° around Y.
3. Random Rotation Y 180°, fixed Seed: same angles after save and reopen.
4. One undo removes the whole Replicator.
5. Freeze Replicator gives N−1 instances in a group, at the same transforms.
6. Viewport Replicators None draws boxes; All draws shaded copies.
