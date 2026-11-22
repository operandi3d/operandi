# Projections, culling and falloff (stage 3): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi projections-stage3 <commit>.app`), on a throwaway
scene with a cube, a sphere, a cylinder, and any picture with readable text.
Each line: do this → expect that. Render = File > Render Viewport.

1. Image Map on the cylinder, Texture Locator tab: Projection Cylindrical,
   axis Y → render: the image wraps once around, its height = Size Y.
   Axis greyed? No (it applies). Projection Triplanar → Axis greyed.
2. Spherical on the sphere, axis Y → the image wraps around, top edge at the
   north pole.
3. Cubic on the cube → each side shows the image; the -Z side is NOT
   mirrored relative to +Z when compared side by side. Rotate the locator
   45° about Y → the projection turns with it.
4. Box on the cube → every side reads right way round from outside. Rotate
   the locator → nothing turns (Box ignores rotation); move/size it → the
   image moves/scales.
5. Triplanar on the sphere, Hardness 0 % → soft blends between the three
   projections; 100 % → hard seams like Cubic.
6. Planar axis Z on the cube, Front Culling: Project Through → the image
   shows on top and bottom; Cull Backfacing → bottom shows the material
   below; Flip Backfacing → bottom shows it mirrored in both directions;
   Flip U / Flip V → mirrored one way.
7. Falloff Spherical, X/Y/Z 0.5, on a large plane → the image fades out
   2 m from the locator centre. Linear X → fades only along X. 0 = no fade.
8. A procedural (Checker) with Box or Triplanar → squares on each face.
9. Random Offsets lists (none), Particles and Replicas, Mesh Parts,
   Surfaces; Texture Offset Strength 0–10; Tangent has dPdu Cross Product
   (all stored only).
10. Save, reopen → every setting comes back.
