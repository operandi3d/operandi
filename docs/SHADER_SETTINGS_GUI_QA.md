# Shader tree remaining, stage 1: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report, on a throwaway scene: a cube, a point or area light, the
Shading panel open. "Render" means Render > Render Viewport. The viewport
itself shows none of this (SR-Q08).

1. Add Layer > Constant above the Base Material, Effect Dissolve, Value
   0.5 → render: the cube is half see-through, shadows half as dark.
2. Same layer, Effect Stencil, Value 0.49 → cube solid; 0.51 → cube gone,
   its shadow gone too.
3. Image Map with a PNG that has transparent areas, Effect RGBA → the cube
   shows the image's colours with holes where it is transparent. Image
   Alpha set to Ignore → no holes.
4. Select the Base Shader → Properties shows Shading, Quality, Visibility,
   Light Linking and Fog sections. Untick Control Fog → the fog rows grey.
5. Fog Type Linear, End 5 m → the far side of the scene greys out.
   Exponential → greys gradually with distance. Fog Start/End grey unless
   Linear; Density greys for Linear.
6. Alpha Type Constant, Alpha Value 40 % → Alpha Value un-greys; render
   with an Alpha output: the cube is 40 % in alpha but still drawn.
7. Item Groups: make a group holding the light. Base Shader Light Group =
   that group, Mode Exclude → render: the cube gets no light from it.
   Mode Include → lit again.
8. Direct Illum Multiplier 50 % → render: the cube is about half as bright
   from the light (sky light unchanged).
9. Add a second cube shadowing the first; Receive Shadows off → no shadow
   on the cubes.
10. Add a second Shader on top, untick its Control Visibility, turn off
    its Visible to Camera → the cube still renders (visibility comes from
    the Base Shader). Tick Control Visibility → the cube is hidden.
11. Each change above is one Cmd+Z step; save, reopen → settings kept.
