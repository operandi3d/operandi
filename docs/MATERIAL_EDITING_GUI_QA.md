# Material editing GUI check (branch material-editing, 2026-11-12)

One pass in the QA app (its own `OPERANDI_DEV_DATA`). Start from a new
scene with the default cube; Shading tab, select **Base Material**.

## 1. Distance fields read units

1. Transparency tab > Transparency: type `25 mm` in **Absorption
   Distance**, Enter. Shows 25 mm (or 0.025 m in metres display), not 25 m.
2. Type `10 mm + 5 mm`: 15 mm. Type `2 cm`: 20 mm. Type `abc`: refused,
   the value stays.
3. Material Ref > Surface Normal: **Bump Amplitude** `1 mm`; Transparency >
   Subsurface Scattering: **Subsurface Distance** `3 mm`. Both keep it.
4. An Image Map layer > Texture Locator tab: **Position** / **Size** take
   `50 cm`. A Backdrop's **Width** takes `1200 mm`.

## 2–3. Value presets (UIC-55)

1. Material Ref > Diffuse and Specular: a small down arrow at the far
   right of **Refractive Index**. Click it: 23 values, Air 1 first, Iodine
   Crystal 3.34 last. Click **Diamond**: the field shows 2.42, one undo
   step.
2. Arrow > **Sort by Name**: alphabetical. Quit and relaunch: still by
   name.
3. Set 1.45, arrow > **(new)**, type `My Glass`, Enter: listed with 1.45.
   **(delete)** > click `Air` > **Done**: gone. **Restore Defaults** brings
   the 23 back (My Glass goes too). Relaunch: changes kept.
4. Transparency > **Dispersion** has the arrow: Flourite 0.007, Silica
   Glass 0.01, Quartz 0.013, Zircon 0.038, Diamond 0.044 (physical B−G
   figures, MAT-ED-Q01). Set Refractive Index to 2.42, pick **Diamond**:
   Dispersion goes to 100 % and Abbe Number to about 55, one undo step.
   With Refractive Index 1, a pick says it needs an index above 1.
   **(new)** saves the material's current figure (Diamond's shows 0.044).

## 4. Tabs and sections

1. With Base Material selected the Properties panel shows six tabs down
   its right edge: Material Ref, Reflection, Transparency, Rays, User
   Channels, Tags. The active name is in the accent colour.
2. Material Ref: Name at the top, Enable/Invert/Blend Mode/Opacity, then
   sections **Diffuse and Specular**, **Surface Normal**, **Luminosity**,
   **Viewport Display**. Each bar is darker with a triangle.
3. Click a section bar anywhere: it folds. Relaunch: still folded.
4. Other tabs: Name at the top of each, their sections below. User
   Channels and Tags say "None yet".
5. Select a Texture layer, then Base Material again: the tab you were on
   stays. Turn Transparent Amount to 0: its dependent rows grey in place.

## 5. Image dropdown thumbnails

1. Add an Image Map layer and load two pictures through **Load Image…**.
2. Open the Image dropdown: each name has a small picture beside it
   (may fill in a moment after the first open). The closed field shows the
   chosen picture at its left.
3. Double-click the Image field: the Image Viewer opens on that image.
   Again: no second copy in the strip.

## 6. Double-click an image layer row

1. Double-click the Image Map's row in the shader tree: the Image Viewer
   shows its image. No rename field appears.
2. Double-click a material group row: it selects the group and its
   children (UIC-38); the viewer doesn't open.

## 7. Preset drop on a group row

1. Open the Preset Browser (F6), drag a material preset over the shader
   tree. Over a group row the row gets an orange outline; over a
   Material/texture row or empty space, none.
2. Drop on the group: its contents are replaced by the preset's layers;
   Cmd+Z puts the old contents back in one step.
3. Drop on a non-group row: nothing happens.
