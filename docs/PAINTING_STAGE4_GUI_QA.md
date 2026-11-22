# 3D painting stage 4: GUI check (tips, Tablet Nozzle, brush presets)

Build: "Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds.
Mouse buttons: **left** = primary, **right** = secondary.

Setup: File > New. Create > Sphere. Texture > Add Color Texture…
(Resolution 1024, Set Color on, black). Texture > Paint Tools > Paint Tab,
pick Paintbrush.

1. **Procedural.** Brushes > Procedural, Size 80. **Left-drag** across the
   sphere: the stroke is patchy. Type Cellular: another pattern. Tick
   Invert: light and dark swap.
2. **Sphere.** Brushes > Sphere: each dab is a shaded round blob with a
   highlight; Flatness 100 % makes it flat.
3. **Texture tip.** Load any picture as an image (File > New Image or an
   Image Map's Load Image…). Brushes > Texture, Image: that picture, Size
   40. Mode Normal, **left-drag**: the picture is laid at each dab. Mode
   Roller: the picture repeats along the stroke like a roller. Mode Stamp:
   **left-press** where the centre goes, drag away (a ring shows the size,
   a handle the angle), release: one copy at that size and angle.
4. **Stamp.** Brushes > Stamp: as Texture's Stamp mode.
5. **Text.** Brushes > Text, Text "TEXT". **Left-press**, drag to the
   right and round, release: "TEXT" is stamped at that size and angle.
   Bold and Italics change it.
6. **Tablet Nozzle.** Click Nozzle (it lights; Tool Properties shows the
   Tablet Nozzle rows). Tick Enable Jitter (the jitter rows stop being
   greyed), Position 50 %: a stroke's dabs scatter off its path. With a pen:
   Strength Pressure lightens light presses; Size Pressure narrows them.
7. **Brush presets.** Set a Procedural tip with some values and the
   nozzle, click Save Brush Preset. Change the tip, Size and nozzle, pick
   another tool. Texture > Paint Tools > Brush Presets > the saved one:
   every tip and nozzle value comes back; the tool doesn't change.
8. **Sculpt tab.** It shows a "Brushes and Nozzle" section with the new
   tips, Save Brush Preset, Nozzle and Inks, all greyed. Sculpt brushes
   work exactly as before.
