# Blender materials starter pack — GUI check

QA build with Cycles: "Operandi blender-materials <commit>.app". Presets
are in `~/Developer/Operandi-Content/Starter/<Name>/<Name>.oppreset`
(32 CC0 materials, maps next to each preset, CREDITS.txt at the top).
Use the QA app only (it reads preset version 37; older builds refuse
these files cleanly).

1. New scene, add a Sphere (UVs on) and a Cube. In the Shading panel add a
   Material Group and select it.
2. File > Load Preset… > `Starter/plywood/plywood.oppreset`. The group now
   holds Base Color, Roughness, Normal, Height layers and a Material; the
   Texture Group has one locator. One Cmd+Z removes it all.
3. Open the Preview (Cycles). Wood grain shows on both shapes; nothing is
   black; the surface is matte, not glossy (roughness not flipped).
4. Load `Bricks113A`. Mortar lines read as grooves under a light from the
   side (normal not inverted).
5. Load `Fence006`. Only the wire mesh renders; the holes are see-through.
6. Load `Metal049A`. It renders as a polished mirror metal (it reflects the
   studio grey; that's expected).
7. Load `metal_plate` (Poly Haven). Dark AO in the crevices, shiny metal
   plates.
8. Move the Starter folder somewhere else and load a preset from the new
   place: the pictures are still found (paths are relative to the preset).
9. Save the scene, reopen it: the material looks the same.
