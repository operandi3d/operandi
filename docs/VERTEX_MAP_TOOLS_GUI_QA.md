# Vertex Map Tools and falloff — GUI check (spec vertex-map-tools-2026-11-11)

For Jason or Codex. QA app named in the thread, own data folder. New scene,
a subdivided Plane, Sculpt tab.

1. **Section.** Below the sculpt brushes: Vertex Map Tools with Paint,
   Smooth, Scale Down, a divider, Erase, Set Value, Live Deformers (on),
   Weight Map (none) and New Weight Map….
2. **First stroke.** Turn on Show Weightmaps and set Paint Value to 20 %.
   Paint on the plane with no map: a map named Weight appears and is
   selected. Sweep back and forth over one point before release. The point
   reaches about 20 % with full brush coverage and keeps its normal dot
   color. A second stroke over it reaches about 36 %. Undo removes one
   stroke at a time.
3. **Tools.** Ctrl Paint lowers the weights. Smooth blurs a painted edge
   (Ctrl sharpens). Scale Down fades it slowly. Erase clears weights.
   Set Value with Value 50 % writes a flat value across the whole
   brush, no fade.
4. **New Weight Map…** with Init Value 25 % adds a map with every point
   set; it becomes the selected one.
5. **Mask.** Falloff menu → Vertex Map, then Add Falloff → Invert.
   Push over the painted and unpainted areas: painted points stay put.
6. **Mix Mode.** Falloff Linear, Add Falloff → Vertex Map, try each Mix
   Mode; the falloff dots change as the spec table says.
7. **Live Deformers off**: the weight shading updates on release, not during the
   stroke.
