# Procedurals stage 4 (Cellular, Wood, Ripples, Weave, variations): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi procedurals-stage4 <commit>.app`), on a throwaway
scene with a Unit Plane (locator axis Z for a plane facing up) and a cube.
Render = File > Render Viewport.

1. Add Layer > Textures lists Checker, Grid, Dots, Noise, Cellular, Wood,
   Ripples, Weave, none greyed.
2. Cellular → render: black cobblestones (Cell) with white gaps (Filler),
   about one per metre. Type Round → soft round spots. Frequencies 3 →
   finer cells layered in.
3. Cellular, Hue Variation 100 % with both zone colours red → each cell a
   different hue; Value Variation → different brightness; Saturation →
   some greyer.
4. Wood → grey rings around the locator axis about 25 cm apart, wavy, with
   fine grain. Waviness 0 and Graininess 0 → clean circles.
5. Ripples → white crests / black troughs from 8 centres, 20 cm apart.
   Drag Phase → waves move (positive inward).
6. Weave → white yarn strands over/under, black gaps; Yarn Width 100 % →
   no gaps; Roundness 0 → flat strips; use Effect Bump to see the tubes.
7. Checker with Alpha Variation 100 % → some squares fade toward the
   material below.
8. Texture Locator tab, uniform Size 1 m, Auto Size on a 2 × 1 × 0.5 box →
   Size 2 m on all three axes; with Size 1 × 1 × 2 first → 2 × 1 × 0.5.
9. Save, reopen → all settings back.
