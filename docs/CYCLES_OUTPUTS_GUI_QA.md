# Cycles built-in render outputs: GUI checklist (2026-11-20)

For Jason or Codex (Claude can't click in the app). QA app: the
cycles-render-outputs build. Start from a scene with two boxes, one with a
UV map, items named "Left" and "Right", the right one with material tag
"Metal".

1. Shading panel > Add Layer > Render Output lists Illumination (Direct),
   Illumination (Indirect), Illumination (Total), UV Coordinates, Segment ID
   and Cryptomatte; each adds "<name> Output" at the top of the tree.
2. Select a Cryptomatte output: Cryptomatte Type (Object / Material /
   Asset) and Cryptomatte Levels (2–16, stays even) respond; Colorspace is
   greyed and its tooltip says why. On any other output the two
   Cryptomatte rows are greyed, Colorspace responds.
3. Render (F9). In the render window's output picker:
   - Illumination (Total): grey-white light on both boxes, no colour.
   - UV Coordinates: the UV box red/green by its UVs, the other black.
   - Segment ID: each box one flat, bright colour; render again: same
     colours. Save, reopen, render: same colours.
   - Cryptomatte (Object): each box one colour; Material: changes when the
     boxes' tags differ.
4. Depth or Normals output: shows its raw numbers even with the window's
   view set to sRGB. Set an Illumination output's Colorspace to Linear: it
   now shows raw too; set it to sRGB and switch the window's view to
   Linear: it still shows through sRGB.
5. Save Image… > Format OpenEXR Multilayer: the Output row greys; OK and
   save. Open the file in Nuke / Blender / an EXR viewer: the main RGBA,
   one layer per output, CryptoObject00–02 (default 6 levels), and the
   Cryptomatte picker in Nuke / Blender picks each box by name.
6. Pick an older slot in the history strip and save it as OpenEXR
   Multilayer: its Cryptomatte layers are there too.
7. Undo / redo a Colorspace and a Cryptomatte Levels change: one step each.
   Save the scene with a Cryptomatte output and reopen: settings kept.
