# Viewport materials — GUI check

QA app, a scene with a sphere and a cube, viewport style **Advanced**.
Compare each step with a Cycles render (F9) of the same scene.

1. Default material: the dark side and the side facing the view light look
   as before; the highlight is broader than it used to be.
2. Material Roughness 0.1, then 0.8: the highlight gets small and bright,
   then wide and dim.
3. Metallic 100 % with a gold Diffuse Color: no flat diffuse, the gold
   shows in the reflection; Specular Color tints its rim.
4. Clearcoat 100 % with Clearcoat Roughness 0: a second sharp highlight.
5. Luminous Intensity 1 with a red Luminous Color: the surface glows red
   on the dark side too.
6. Image layer on Roughness (any picture): the highlight changes per pixel.
7. Image layer on Bump: the surface looks embossed and the relief moves
   with the light; set Bump Amplitude to 0 and it goes flat.
8. Normal map image (tangent space): the relief matches the render's.
9. Image layer on Stencil: the parts above 50 % disappear.
10. Switch to Default or Texture: everything looks as it did before.
