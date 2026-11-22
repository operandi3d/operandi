# Shader tree remaining, stages 2 onward: GUI checks

For Jason or Codex. QA build named in the report, throwaway scene: a
sphere and a cube, the Shading panel open. "Render" = Render > Render
Viewport. The viewport doesn't show these looks.

## Stage 2: Gradient and Drivers

1. Add Layer > Processing > Gradient above the Base Material → Properties
   shows Input Parameter (Incidence), a white Color strip and one key.
   Render: surfaces flat white.
2. Add Key; set the second key to 90° and black → render: the sphere is
   white facing you, darkening toward its edge.
3. Drag a key along the strip → the strip and key list follow; one Cmd+Z
   undoes the whole drag. Middle-click (or Ctrl+Alt+click) on the strip →
   a key appears there.
4. Set a key's shape to Stepped → the strip shows a hard step.
5. Input Parameter menu: groups Basic Channels, Sample Parameters, Shader
   Control, Special Effects, Surface Shading, Particle Effects; stored
   inputs say "(stored)".
6. Effect Roughness: the strip shows the Value curve with percent values.
7. A Constant layer with Effect Driver A, Value 30 %, below a Gradient
   with Input Driver A → the surface changes only through the Gradient.
8. Slope input on the cube, keys 0° white, 90° black → top white, sides
   black.
9. Save, reopen → keys, shapes and input kept.
10. (2026-11-09) A Cube primitive (it has the "Texture" UV map), Gradient
    Input Texture U, keys 0 black and 1 white, Effect Luminous Color →
    render: each face runs dark to light across its tile of the UV layout.
    On a mesh with no UV map the render is flat and the status line says
    "A Texture U/V gradient reads 0 on a mesh without UVs".
11. Input Screen X, keys 0 black and 1 white, on a plane filling the
    camera view → render: black at the left edge, white at the right.
    Screen Y: black at the bottom. Screen R: black in the middle, white in
    the corners. The menu no longer marks Screen X/Y/R "(stored)".

## Stage 3: Vertex Map and Variation textures

1. Make a weight map on the sphere (paint part of it). Add Layer >
   Processing > Vertex Map Texture, pick the map → render: zone 1 (black)
   where the weight is 0, zone 2 (white) where 1. Change Color 1 / 2 and
   Bias → the render follows.
2. Pick a colour map instead → the render shows the map's colours.
3. A second item without that map → unaffected.
4. Variation Texture, Source Item, Color keys black → white; duplicate the
   cube a few times → each copy a different grey. Source Mesh Part on one
   mesh with separate pieces → each piece its own grey.
5. Source Particle shows "(stored)" and the note; the render is flat.

## Stage 4: Process and Occlusion

1. Image Map on Diffuse Color, then Add Layer > Processing > Process
   directly above it → no change. Hue Shift 90° → the image's hues turn;
   Saturation −100 % → grey; Opacity 50 % → halfway.
2. Move the Process above the Base Material instead → no effect.
3. Occlusion on Diffuse Color, cube on a floor → dark where they meet,
   white in the open. Occlusion Distance 1 m → darker, wider. Rays 4 →
   noisier.
4. Type Up → the note "renders as Uniform"; Thickness → thin parts dark.

## Stage 5: group Scale, Apply to Sub-Group, Material extras

1. A Material Group with an Image Map and a Material inside. Scale 50 % →
   render: the image tiles twice as often; Displacement Distance halves.
2. Tick Ignore Scale Group on the Image Map → its size is back.
3. Apply Scale → Scale reads 100 %, the material's distances are halved,
   the render looks the same; one Cmd+Z restores both.
4. An untagged group with Apply to Sub-Group on, a texture in it and a
   group for material "Red" inside → only Red polygons get the texture.
5. Material: Use Refractive Index on, Refractive Index 2.4 → stronger
   reflections. Rounded Edge Width 2 cm on a cube → softened edges.
   Glass (Transparent Amount 100 %), Transparent Color blue, Absorption
   Distance 0.2 m → thick parts deeper blue.

## Stage 6: Render Outputs

1. Add Layer > Special > Render Output lists 18 types; Depth Output lands
   at the top of the tree.
2. Render → the Render window has an output menu next to the buttons:
   Depth shows near surfaces darker... (raw metres: mostly white); tick
   Remap Pixel Values → white near, black at 10 m.
3. Diffuse Color output → flat unlit colours. Shading Incidence → black
   facing you, white at the silhouettes.
4. Final Color output: Gamma 2.2 → the picture brightens; Tone Map Amount
   100 % → highlights roll off; Bloom on → glow round bright spots.
5. In the render folder: "Render N.png" plus "Render N (Depth Output).png".
6. Disable every output → Render says nothing to render.
7. Each settings change is one Cmd+Z step; save, reopen → kept.
