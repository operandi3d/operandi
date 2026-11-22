# Missing tools, group 2: Plane Item and Teapot (v1, 2026-11-21)

Behaviour only. Provenance: the research unit-primitive spec of 2026-11-05 (case UP12, already in Operandi's test data and skipped there), the reference's public guide page for its Teapot primitive, and scripted observation (2026-11-21). REA-assisted: no.

**No reference geometry crosses over.** The teapot shape comes from the public Utah Teapot data (see MT6-06), never from the reference's mesh. Only its size, placement and orientation rules are copied.

## MT5 Plane Item
- MT5-01 One click, no tool, no drag: adds a new mesh item named "Plane" at the scene origin (no transform) and makes it the only selected item. The previously selected mesh is unchanged. Same family as Cube Item / Sphere Item (Operandi's "New Unit" lines); tool settings are ignored.
- MT5-02 Geometry (exact, from UP12): 4 points (-0.5,0,-0.5), (0.5,0,-0.5), (0.5,0,0.5), (-0.5,0,0.5); one Face polygon [0, 3, 2, 1], facing +Y.
- MT5-03 UVs (observed): a "Texture" map, per corner of that polygon in its order (0,1), (0,0), (1,0), (1,1); i.e. u = x + 0.5, v = 0.5 − z (looking down from +Y, −Z at the top of the UV square).
- MT5-04 Where it lives: Shift+click of the Text button. Operandi's Text button has no variants yet, so add a Shift variant "Plane Item" there (tag "Item", same tooltip pattern as Cube Item). Also a command line name `Plane Item`.
- MT5-05 Un-skip UP12 in unit_primitive_clean_spec.rs; it is the acceptance case.

## MT6 Teapot (primitive tool) and Teapot Item
Tool settings (defaults): Position X/Y/Z 0 = centre of the teapot's bounding box; Size 1 m = length of the bounding box's longest side (spout tip to handle); Axis Y = which world axis is up (X, Y or Z); Make UVs on = adds a "Texture" UV map.
- MT6-01 Proportions (Axis Y, Size 1): bounding box X ±0.311061, Y ±0.244961, Z ±0.5. Exactly the classic data's 6.434 : 4.0 : 3.15 (length : width : height) scaled so length is 1. The classic lidded shape with the usual 0.75 vertical squash, not the taller 1975 original.
- MT6-02 Orientation (Axis Y): up is +Y, the spout points −Z, the handle +Z, symmetric across X = 0. Lid knob top at (0, +H/2, +0.0334·Size), bottom pole at (0, −H/2, +0.0334·Size): the pot's centre line sits 0.0334·Size toward the handle because the spout reaches further than the handle. Axis X / Z rotate "up" onto that axis.
- MT6-03 Closed, rounded bottom (a single bottom pole point), and a lid. One mesh; the lid is not a separate piece.
- MT6-04 Quads with triangles only where patches meet at a pole (knob top, bottom, spout/handle ends). Operandi picks its own density; reference counts deliberately not copied. Suggested: 6×6 quads per patch, poles as triangles, coincident patch edges welded so the mesh is connected with no duplicate points. Face polygons.
- MT6-05 Teapot Item: one click, new mesh item named "Teapot" at the origin with the defaults above, selected; previous selection unchanged (same rules as MT5-01).
- MT6-06 Source data and licence: the 32-patch version with the rounded bottom (Crow 1987; classic lidded shape, 4×4 control points per patch) from https://graphics.cs.utah.edu/teapot/ — free for any use, commercial included, credit as "Utah Teapot". Put that credit in the About box credits and the data file's header. Store control points as a small data file in our own format.
- MT6-07 Safe to ship, with the credit. Not safe: copying the reference's teapot mesh, point counts or file.
- MT6-08 Keep Operandi's Shift+Ellipsoid = Ellipsoid Item. Put Teapot (tool) and Teapot Item in the Geometry menu's primitive list and the command line, no toolbox button (toolbox spot is Jason's call).
- MT6-09 Like other primitives: works with the unit/new-item rules, symmetry and Make UVs. UV layout is Operandi's own (per-patch u/v from patch parameters, packed).
