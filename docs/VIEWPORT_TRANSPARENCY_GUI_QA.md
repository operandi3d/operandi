# Viewport transparency — GUI check

QA build from branch `viewport-transparency`. Views in Advanced style
unless a step says otherwise (transparency shows only there, Jason
2026-11-14).

1. **VT-01** Two cubes, the second behind the first and red (Diffuse Color
   red). Give the front cube a material with Diffuse Amount 0 %,
   Transparent Amount 98 %, Transparent Color white. The red cube shows
   clearly through it, and the front cube's edges and silhouette show as a
   faint light rim.
2. **VT-02** Set Transparent Color amber: the red cube now shows through
   amber-tinted.
3. **VT-03** Transparent Amount 0 %, Diffuse 80 % white: plain opaque white,
   as before.
4. Try the other styles: Shaded, Texture, Default, Solid, Gooch, Cel and
   Matcap draw the glass solid, light grey (not black).
5. **VT-04** Whiskey glass scene: glass, liquid and ice all readable through
   each other while orbiting. Make the liquid the active mesh, Polygons
   mode: clicking liquid polygons through the glass selects them, and the
   selection and wire show on top.
6. Select glass polygons: they draw solid in the selection colour.
7. **VT-05** Preferences > Display > Show Material Transparency off: the
   glass draws solid light grey (tinted by Transparent Color), never
   black. On again: see-through.
8. **Shadows** (Advanced, shadows on): a cube in front of a wall, lit so
   its shadow falls on the wall. Transparent Amount 0 %: a full shadow;
   50 %: about half as dark; 98 %: only faint. Show Material Transparency
   off: a full shadow again. The lighter shadows look even, not grainy.
9. Speed: a big opaque scene (10M+ triangles) orbits as fast as on main;
   only items with see-through materials draw twice.
