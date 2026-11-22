# USD import (U1, U2): GUI check

For Jason or Codex. QA build "Operandi usd-import <commit>.app" from branch
`usd-import` (USD and Cycles on). Use the QA app's own data folder. Test
files are in the branch's `fixtures/usd/usd_t1/`.

1. File > Open… `usd_t1.usdc`. The **Import USD** dialog opens first: Up
   Axis, Units and Import Materials (From File, From File, MaterialX),
   Import Cameras, Import Lights and Use Graphs Sidecar ticked, Graphs File
   empty (greyed while the sidecar is off, UIC-92), Instances on
   "Instances". Cancel: nothing
   changes, the status line says so.
2. Open it again and press OK. The scene opens unsaved, with a report
   window: "27 item(s), 512 polygon(s): 480 quad(s)…", materials by kind,
   10 instances as copies, 1 camera, 3 lights and the dome imported.
3. Item list: Base ▸ Arm ▸ Hand (three levels), UVCube, SubD, GlassCube,
   Emitter and ChairInstance0..9 each holding Chair_0. Select Base: its
   Rotation is 30° about Y (in order XZY), Position 3, 0, 0. Everything
   stands upright on the grid (the file was Z-up).
4. Polygon mode on UVCube: six quads; the UV view lists Texture and UVMap2;
   the vertex colour map Col shows red/green/blue/yellow corners.
5. SubD is smooth (Catmull-Clark) with four sharp top-ring edges.
6. Move one chair: only that chair moves. In Polygon mode move a point of
   any chair's mesh: all ten chairs change (Mesh Instances); Undo puts all
   ten back. Save, reopen: still linked. Reopen the .usdc with Instances =
   Copies: editing one chair changes only that one.
7. Shading panel: groups Painted, Glass, Glow. Painted holds Diffuse
   Color, Roughness and Normal image layers. In Advanced shading or Cycles
   preview the UVCube shows the gradient picture (not washed out), the
   sphere glows orange, the glass cube is clear.
8. File > Open… `usd_t1.usdz`, OK: the same scene; with a project set the
   pictures are unpacked into its `sourceimages/usd_t1/` folder.
9. Copy `usd_t1.usdc` alone to another folder and open it: it still
   imports, UVCube shows the missing-picture look, and the report lists
   the three missing pictures.
10. Reopen `usd_t1.usdc` with Import Materials = None: polygons keep their
   Painted/Glass/Glow tags, no Material Groups. Units = Centimeters: the
   scene is a hundredth the size.
11. File > Export ▸ USD Binary of the imported scene, then open that file:
   the same items and groups come back.
12. Camera view through Camera: the chairs run from the lower left, the
   cube in the middle, as in Blender's camera view. Render: the sky
   picture behind, the sun, point and area lights lighting the scene about
   as bright as Blender's render of the same file.
13. Reopen with Import Lights off: no lights, the default environment.
14. Open `fixtures/usd/usd_t4/usd_t4.usdc` (its `usd_t4_graphs.json` sits
   beside it): the report names "Node graphs from usd_t4_graphs.json:
   Marble"; the Marble group holds one Node Material layer and the cube
   renders a dark-and-cream noise marble, not flat grey. Untick Use Graphs
   Sidecar and reopen: the plain material instead.
