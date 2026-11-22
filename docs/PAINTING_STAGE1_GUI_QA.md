# 3D painting stage 1: GUI check

Build: a painting QA app (`--features painting`), named
"Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds. A normal QA
or release build must show none of this (File and Texture menus as
before).

Start with a new scene, Create > Cube (it has UVs).

1. **File > New Image…**: panel with Name, Resolution (2048), Format
   (RGBA), Floating Point (off), Set Color (off), Color greyed until Set
   Color is ticked. OK: no undo step; Texture > Images lists it with "*".
   Resolution 300: status says "Painting works best on square and power of
   2 images." once.
2. **Texture > Add Color Texture…** (a Material Group selected in the
   Shader Tree): the panel, then an Image Map at the top of the group with
   effect Diffuse Color and the paintbrush beside it. Cmd+Z removes the
   layer; the image stays in the Images window.
3. **Add Mask Texture…** with a layer selected: lands directly above it,
   effect Layer Mask. **Add Displacement Texture…**: effect Vector
   Displacement, float image. Add Fur Textures… is greyed "Not yet".
4. **Refusals**: select a non-image layer, type `operandi.paint.target`:
   "No texture selected. Add a new texture?" (Add/Cancel). Hide the image
   layer: "Nothing to paint on…".
5. **Texture menu**: Invert Image, Clear Image, Set Resolution…, Expand
   UV Seams each one undo step; the cube shows the change before saving.
   Flatten Layers with two Image Map layers selected: one new layer above,
   the two turned off. Convert to Vector Displacement greyed.
6. **Saving**: File > Save Image on a new image opens the save panel
   (PNG first, OpenEXR for float images; PSD/PSB offered). The "*" goes.
   Save the scene: changed images are saved too (File > Scene Save Also
   Saves Changed Images ticked). Quit with a changed image: the list
   question (Save / Don't Save / Cancel).
7. **PSD**: Save Image As… a .psd. The Image Map shows a PSD Layer row
   (Composite, Layer 1). Invert Image on the Composite asks "Paint into
   which layer?"; New Layer adds "Operandi Paint 1". Save, open the file in
   another app: layers and the composite are there. Change the file
   outside Operandi with no unsaved paint: it reloads; with unsaved paint:
   "Reload from disk and lose unsaved paint?".
