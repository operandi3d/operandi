# 3D painting: where it stands (2026-11-18)

> **In development, on by default** (since 2026-11-19): dev, release and CI
> builds include painting. `--no-default-features` (or
> `OPERANDI_PAINTING=0 scripts/bundle-dev.sh`) builds without it. Buttons
> for parts not built yet are greyed and say "Coming in
> a later stage"; open choices are the PNT questions in
> docs/SPECIFICATION_QUESTIONS.md.

Normal and release builds don't have it. To try it, open "Operandi
PAINTING <sha>.app" in ~/Developer/Operandi-Builds. Stages 1–6, the
painting icons and the fixes since (stale Shader Tree selection hang,
Sub-D painting) were merged into main together on branch painting-icons.
Each stage has a GUI checklist in docs/PAINTING_STAGEn_GUI_QA.md. The
questions are in docs/SPECIFICATION_QUESTIONS.md under "3D painting stage n".

## Quick start

1. Create > Sphere.
2. Texture > Add Color Texture… (Set Color on, black).
3. Paint tab > Paintbrush.
4. Left-drag on the sphere.

If nothing paints, a dark box at the top of the view says why. Or use the
Paint layout from the layout switcher.

## What each stage built

| Stage | Built | Not built / held back |
|---|---|---|
| 1 Images | New Image, Add Color/Mask/Displacement Texture, the Images window, PNG/EXR/TGA/TIFF/JPEG and PSD/PSB with their layers, Save / Save As / Save All, Expand UV Seams, Flatten Layers, reload when a file changes outside Operandi | Image sequences, fur textures, vector displacement |
| 2 Brushes | Airbrush, Paintbrush, Eraser; Smooth (with Custom profile) and Hard tips; 14 blend modes, Opacity, Density, Attenuation, falloff, symmetry; one undo step per stroke | Changed-tile GPU uploads |
| 3 More tools | Smudge, Blur, Sharp, Clone, Line/Circle/Rectangle, Fill, Linear/Radial Gradient, Lasso | — |
| 4 Tips | Procedural, Sphere, Texture (Normal/Roller/Stamp), Stamp and Text tips; Tablet Nozzle; brush presets (.opbrush) | Sculpting with the new tips (greyed) |
| 5 Inks | Image Ink (widget; E/R/D/F turn and size it), Parametric, Random and Projection Inks, which stack; Curve Constraint (Background 2D/3D) | Inks on sculpt brushes (greyed); Parametric's Displacement and Tangent Pressure (greyed) |
| 7 Sculpt extras | Procedural/Sphere/Texture/Stamp/Text tips, the nozzle and Image/Parametric/Random Inks on sculpt brushes; mask (5 mask brushes, red display, Enable/Invert/Opacity, Clear, Bake Falloff); seven starter brushes with our own pictures in the strip; Geometry to Brush; Constrain to Background on sculpt | Vector-displacement tips; stamp-once on sculpt; nozzle rotation on sculpt; Copy to Morph (no morph maps) |
| Icons + fixes | 36 approved painting icons; Add Color Texture hang after closing a scene; Sub-D meshes paint the drawn smooth surface | — |
| 6 Layout | Paint layout with the Sculpt/Paint tabs and a brush preset strip; Utilities tab; Texture > Image Processing; Images & Painting and Default Tools preferences; painting in the UV view; Image Viewer with PSD layer picker; flyouts; Ctrl+Shift+click colour pick; lasso handles, fixed to the model; viewport shows a picked PSD layer | Lasso on the falloff stack; preferences aren't saved yet |

## Guesses waiting for an answer from Research

These are the choices made where the spec doesn't say. Each one is marked
provisional in the code.

- **Command names** are Operandi's own (`operandi.paint.*`,
  `operandi.image.*`) (Q01).
- **Preferences** aren't saved between sessions yet (Q06, Q54).
- **The stroke** shows in the view about 30 times a second, and the whole
  image is re-sent each time (Q19, Q58).
- **Where a press paints:** the textured mesh under the pointer, even if it
  isn't selected. With no Image Map layer selected, the top layer that has
  an image is the target (Q40).
- **One shared set of inks** serves all the paint tools, like the tip. On
  Eraser, Clone, Smudge, Blur and Sharp an ink changes only the strength
  (Q41).
- **Image Ink's keys:** E/R turn the picture 5° and D/F size it 5 %. While
  the picture shows, they take over Rotate, Scale, Subdivide and Flip
  (Q44).
- **Projection Ink** uses a selected camera, or else the render camera
  (Q45).
- **Curve Constraint:** every visible, unselected curve counts. With none,
  the stroke paints freely and says so (Q48).
- **Brush preset strip** is drawn over the bottom of the 3D view, and its
  thumbnails are simple tip discs (Q50).
- **The Sculpt/Paint tabs** replace the usual toolbox tabs only in the
  Paint layout (Q51).
- **Painted images** open in their own Image Viewer window, not the Render
  window (Q53).
- **Colour pick** takes the image's colour, not the lit colour on screen
  (Q55).
- **UV view:** while a paint tool is on, left-drags paint instead of
  selecting (Q56).
- **The lasso** stays painting's own rather than joining the falloff stack.
  Once the view moves it is fixed to the model and its outline hides (Q57).

The full list, with every number, is in SPECIFICATION_QUESTIONS.md.

## Safe to know

- Default builds are unchanged. Their tests pass, and so does
  `scripts/check-no-painting.sh`.
- Sculpting is unchanged. The shared brush engine is held by
  `tests/sculpt_brush_engine.rs`.
- Undo takes back strokes one at a time but never deletes an image you
  made (PNT-11).
- Sub-D: the stored (CPU) Sub-D surface now carries the UV maps, so
  paint and image textures show on small Sub-D meshes during a stroke.
  The same fix is on `fix-subd-stored-uvs` for normal builds.
- While a paint tool or sculpt/mask brush is the tool in use, no element
  highlighting draws (selected tint, outline, selected edges/points,
  hover). Any other tool brings it back. Painting builds only.
- The Model layout's strip has no Paint tab; the Paint layout's set has
  Paint Tools, and Texture > Paint Tools > Paint Tab opens it anywhere.
- Stroke start (2026-11-18): the surface's texels (which pixel lies where)
  are kept per mesh, Sub-D level, UV map and image size and made while the
  tool is on, a press places them in the view one 64-point tile at a time
  as dabs reach it, and the 3D views' texture takes only the changed
  rectangle (PNT-Q58). Release build, Sub-D sphere, 2048 image: press
  260-310 ms -> ~10-13 ms, drag frames ~51 ms -> ~3-6 ms, release ~48 ms
  -> ~8-10 ms. Probe: `painting_speed_tests::paint_stroke_speed_probe`.
- Texture Locators don't draw while a paint tool or brush is in use;
  Texture > Paint Tools > Show Texture Locators while Painting (off) brings
  them back.
