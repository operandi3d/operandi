# New scene — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05. REA-assisted: no.

## Rules

1. **Scene items** (NS01): one mesh **Mesh** (0 points, 0 polygons), one
   camera **Camera**, one directional light **Directional Light**, plus the
   render and environment items (Render, Environment) and their shader tree
   (Base Material, Base Shader, Final Color Output, Alpha Output; Light
   Material under the light; Environment Material under the environment).
   No other meshes, no geometry.
2. **Selection:** the mesh is the only selected item; Item selection mode.
3. **Transforms** (NS02): Mesh at the origin, no rotation, scale 1.
   Camera at (0, 0.75, 4) rotated −5° about X. Light at (−2, 2, 2) rotated
   −30° about X and −45° about Y. Rotation order ZXY for all three.
4. **Camera:** perspective, focal length 50 mm, film 36 × 24 mm.
   **Light:** radiance 3, ray-traced shadows. **Render:** 1920 × 1080,
   frames 1–150.

## Open questions and decisions

- **Q-NS-01** Which of these come from preferences or a default scene
  template (Modo allows one), and which are fixed.
- **Q-NS-02** Environment and render settings beyond resolution and frame
  range; Operandi may not need them.
