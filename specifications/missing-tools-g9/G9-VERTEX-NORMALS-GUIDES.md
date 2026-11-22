# G9 Vertex Normal tools and Guides
REA-assisted: no. Sources: the reference's plain-text option lists and preference defaults, and scripted tests (2026-11-21). No internal names, screenshots or vendor files carried over.

## Part A: Vertex Normal Tools
Operandi already has: vertex normal maps (stored, saved); Split Normals on selected edges. Everything below is missing.

VN-0 Where it lives: a Vertex Normal Tools palette, opened from the Model toolbar button of that name and a menu item "Vertex Normal Tools...". Floating panel, Operandi-styled per the UI bible, three tabs: Smoothing (VN-1..VN-6), Tools (VN-7..VN-10), Cage (VN-11).

VN-1 Hard edges stored as an edge set (observed): hardened edges go into an edge selection set "HardEdges" on the mesh. The vertex normal map ("Vertex Normal") is rebuilt from that set. Save/load keep both.
Smoothing rule: a corner's normal is the average of the face normals around that point reachable without crossing a hard edge. One hard edge alone doesn't split a corner if you can get round it the other way.
Cube oracles: all 12 edges hard: every corner of the −Y face reads (0, −1, 0). Then soften edge 0–1 (shared by −Y and −Z faces): on −Y face its two end corners read (0, −0.707, −0.707); the −Y face's other two corners stay (0, −1, 0); on −Z face the matching corners read (0, −0.707, −0.707), its far corners (0, 0, −1). No hard edges: every corner reads ±0.577 on all axes, fully smooth even at 90°.

VN-2 Recalculate Vertex Normals: rebuilds the normal map for selected meshes, or creates it if missing. Smooths everywhere except across hard edges. With "Harden Smoothing Groups" on, also splits at smoothing-group borders. Weighting from Options (VN-6). Re-runs whenever an option changes.

VN-3 Harden Vertex Normals: adds edges to HardEdges, then rebuilds. Mode: Vertices = edges at selected points; Edges = selected edges; Polygons = outer border of selected polygons; By Angle = selected edges (all when none) sharper than Angle (default 40°); UV Auto Boundaries = UV seams of the current UV map, or polygon UV-island borders in polygon mode; UV Edge Boundaries = edges on UV seams; UV Polygon Boundaries = borders of the selected polygons' UV islands. Switches: Soften Other Edges (off): every other edge becomes soft. Connected Verts (off): in Vertices mode, harden only edges with both ends selected. Observed: By Angle with all edges selected on a cube hardens all 12.

VN-4 Soften Vertex Normals: removes edges from HardEdges and rebuilds. Modes Vertices, Edges, Polygons, plus Connected Verts. A plain Soften button softens every edge.

VN-5 Select Hard / Soft Edges: Select = Hard Edges or Soft Edges; Mode = Set / Add to / Remove from Selection. Observed: after hardening one edge, Select Hard selected exactly that edge.

VN-6 Options (preferences): Use Custom Normal Map Name on; Angle 40° (0–180); Area Weighting on; Sum Co-Planar Areas on; Angle Weighting on; Coplanar Tolerance 0.005 rad (0–5°); Full Weighting off; Harden Edges on (Recalculate respects HardEdges); Harden Smoothing Groups on; High Quality Normals on.
Weighting (PROVISIONAL VN-Q1): each face normal counts by area x corner angle; Blender's Weighted Normal modifier is the model. Sum Co-Planar Areas on: faces flat within Coplanar Tolerance count as one face with combined area. Full Weighting off = a corner takes only the largest-area facing direction when one clearly dominates (guess; check against Blender's Face Influence). Area Weighting and Full Weighting are linked: turning one on turns the other off.

VN-7 Toggle Vertex Normal Maps: Auto (default) removes normal maps if any exist, otherwise recreates; On recreates; Off removes.

VN-8 Transfer Normals: copies normals from the background mesh to the foreground mesh, onto the nearest surface point. World Space (default on, PROVISIONAL) takes item transforms into account. Reuse Operandi's Vertex Map Transfer and remove its refusal for Vertex Normal.

VN-9 Convert to Hard Edges: every edge where shading is split (smoothing angle, smoothing groups, split normals) becomes hard. Remove Smoothing Groups (default off) deletes the groups afterwards.

VN-10 Other tools: Morph to Normals (each corner normal points along the selected morph map's offset at that point). Rename Normal Maps (renames normal maps on selected meshes to the custom name). Spin Polygon Vertex Indices (rotates stored point order of each selected polygon by Offset, default 1 PROVISIONAL, negative the other way; shape unchanged, corner data moves with its points). Bake for Engine: leave out.

VN-11 Cage tab (morph cage view): draws a morph map as a cage over the base mesh, per-item display settings: Draw Base Mesh, Draw Base Mesh Wireframe, Draw Morph Offset Vectors, Shade Base Mesh; a colour and opacity for each. Plus Morph Push tool: Distance; Absolute (off); Push Existing Directions (off); Angle Correction (on, PROVISIONAL). Lowest priority.

Open: VN-Q1 weighting maths and High Quality Normals; VN-Q2 Cage display defaults.
Tests: 1. VN-1 cube oracles. 2. By Angle 40° hardens 12/12 cube edges and 0 of a UV sphere's. 3. Recalculate keeps HardEdges; save/reopen gives the same normals. 4. Select Hard after hardening one edge gives that edge only. 5. Spin Polygon Vertex Indices Offset 1 on a quad: point order rotates by one and UVs follow.

## Part B: Guides (snapping guides)
GD-0 A Guides button in the Snapping group of the Model toolbar (and the snapping popover). A tool that stays on alongside other tools, like a snapping helper.
GD-1 (PROVISIONAL, gestures untested): up to 10 guides, numbered 0–9; Current Guide chooses which you edit. Each has Enable Guide, Show Guide, Snap to Guide (all off by default) and Lock Guide (can't be moved). Mode: Two Points = line through Position X/Y/Z and End X/Y/Z; Planar Ray = line from Position lying in a Plane (YZ, XZ or XY; default YZ) at an Angle in that plane, of a set Distance. Drawing works like the Ruler: drag from start to end. Shows as a thin line in a guide colour, stays after the tool is off while Show Guide is on.
GD-2 Snap to Guide on: points dragged by any tool snap to the nearest spot on any enabled guide within snap range, as one more target in the existing snapping system. Ruler Array guides count too.
GD-3 Guides are per scene and saved with it (PROVISIONAL).
Open: GD-Q1 gestures and Angle/Distance defaults.
