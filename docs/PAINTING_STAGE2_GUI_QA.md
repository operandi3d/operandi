# 3D painting stage 2: GUI check (brush engine)

Build: "Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds
(painting and Cycles). Mouse buttons are named: **left** = primary,
**right** = secondary, **middle** = wheel button. Alt + drag is still
navigation.

Setup: File > New. Create > Sphere. Texture > Add Color Texture…, set
Resolution 2048, tick Set Color, Color black, OK. The new Image Map is the
paint target (paintbrush badge in the Shader Tree).

1. **Toolbox.** The tab strip has no Paint tab yet. Texture > Paint
   Tools > Paint Tab adds it (it stays from then on; turning on a paint
   tool opens it too). Airbrush, Paintbrush, Eraser,
   Smooth and Hard are live; every other button is greyed and its tooltip
   says "Coming in a later stage". Texture > Paint Tools lists the same
   three tools and two tips.
2. **Airbrush, live.** Click Airbrush (it lights; Tool Properties shows
   Airbrush rows, then "Smooth Tip"). Set Size 30. **Left-drag** across the
   sphere: white paint appears while you drag, not only on release. The
   status line reads "Airbrush · 30 px · Image 2048 × 2048". Cmd+Z once:
   the stroke is gone entirely; Cmd+Shift+Z brings it back.
3. **Seam.** Paint a stroke across the sphere's UV seam (the vertical
   line at the back where the texture wraps; turn the view to it): no gap
   and no doubled line on either side.
4. **Back face.** Paint a big stroke (Size 200) on the front, then orbit
   (Alt + left-drag) to the back: untouched.
5. **Size on screen vs model.** Zoom out (wheel), paint with the same
   Size: the band is wider on the model. Tick Auto Scale, paint, zoom out,
   paint again: the band keeps its width on the model.
6. **Ring and sizing.** Hover the sphere: a ring of the tip size follows
   the pointer. **Right-drag** left/right: the ring shrinks and grows (it
   stops at 500 px); a right-click without moving still opens the menu.
   **Middle-drag**: nothing happens. Pick Hard: a second, fainter inner
   ring shows where the Soft Border starts.
7. **Opacity and build-up.** Airbrush, Opacity 50 %: scrub back and forth
   in one stroke: the paint never passes mid-grey. A second stroke goes
   brighter. Paintbrush, Density 20 %: overlapping dabs in one stroke don't
   get brighter where they overlap. Airbrush, Density 20 %: going over the
   same place in one stroke builds up.
8. **Attenuation.** Attenuation Steps 20: a long stroke fades out and
   stops after a short distance; a slow stroke runs out sooner.
9. **Eraser.** On the RGBA texture: **left-drag** with Eraser shows the
   material below through the stroke. Make an RGB image (File > New
   Image…, Format RGB) as the paint target: Eraser paints the BG colour.
10. **Blend modes.** Paint red with each Blend Mode over a gradient: each
    looks like the Shader Tree layer blend mode of the same name; Erase
    takes the paint away.
11. **Sculpt unchanged.** Sculpt tab: Push and Smooth on a mesh behave and
    look as before; switching to a sculpt brush turns the paint tool off and
    back.
