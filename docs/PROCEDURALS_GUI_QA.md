# Procedural textures (stage 2): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi procedurals-stage2 <commit>.app`), on a throwaway
scene with a Unit Plane (and a sphere for the 3D types). Each line: do
this → expect that. Render = File > Render Viewport (Cycles).

1. Shading tab, Add Layer > Textures → Checker, Grid, Dots, Noise; Cellular,
   Wood, Ripples, Weave greyed ("coming").
2. Add Checker → row lands below Base Shader, directly above Base Material;
   Items gains "Checker (Texture)". Cmd+Z removes layer and locator in one
   step.
3. Hover the Checker row → tooltip says the viewport shows the zones'
   average. The viewport shows 50 % grey; Render → black/white squares 0.5 m
   across with soft edges (Transition Width 10 %).
4. Checker Properties: Type, Locator, Checker 1/2 Value, Color, Alpha,
   Bias, Gain, Transition Width, variations. Set Checker 1 Color red →
   Render → red/white squares. Checker 1 Alpha 0 → those squares show the
   material below.
5. Texture Locator tab: Size 2 m → squares 1 m. Projection Planar, axis Z,
   Type Cube → a flat 50 % mix (expected).
6. Add Grid on a plane (locator axis Z for a plane facing up) → thin black
   lines on white, 1 m cells centred on the origin. Try Line, Triangle,
   Hexagon. Line Width 50 % → thick lines.
7. Add Dots → black dots 0.8 m across on 1 m centres. Hexagon → every other
   row shifted. Cube on a sphere → dots of varying size.
8. Add Noise → soft, low-contrast grey blobs about 1 m across. Turbulence →
   creased dark valleys. Simple → Frequencies greys out. Flow Noise on,
   drag Flow Phase → the pattern moves in place.
9. Bias 80 % → brighter overall; Gain 80 % → sharper contrast around the
   middle.
10. Effect Roughness on a Checker → render shows rough/glossy squares.
11. Save, reopen → every procedural setting (including variations and
    Flow) comes back.
