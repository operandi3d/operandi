# Textures stage 5 (Gabor, Poisson, viewport patterns, gizmo): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi textures-stage5 <commit>.app`), on a throwaway scene
with a subdivided plane (many points) and a cube. Render = File > Render
Viewport.

1. Add Layer > Textures ends with Gabor Noise and Poisson Cells.
2. Gabor Noise → render: almost all black (zone 2) with sparse white specks
   (intended). Bandwidth up → more random; Orientation turns the stripes.
   Properties show Brightness and Contrast, no Bias/Gain.
3. Poisson Cells → white evenly spaced cells on black, 4 per metre at
   Level 3; Level 4 → twice as many per side; Cell Density 30 % → smaller
   cells; Solid 3D Cells on a cube → cells carved in 3D.
4. Viewport: add a Checker on the dense plane → the viewport shows the
   squares (blended at the mesh points), not grey. On a single-polygon
   plane it shows a blend (expected; see tooltip).
5. Select the Checker layer (or its locator in Items) → the viewport draws
   the locator: axes and a unit cube (Solid). Projection Planar → a square
   at Size across the axis with a dotted arrow; Cylindrical → a cylinder;
   Spherical → three rings. Deselect → it disappears.
6. Texture Locator tab: Real World Size X/Y, Texture Replicator, Projection
   Camera rows (hover: stored only). Set them, save, reopen → kept.
7. Move the plane item → the viewport pattern moves with it (World
   Coordinates off).
