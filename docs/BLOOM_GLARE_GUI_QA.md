# Bloom and Glare: GUI check (branch render-bloom-glare)

Render window ▸ Image Processing tab. Use the QA build with its own data
folder. A scene with a bright light visible to the camera (an emissive
sphere, a sun glint on chrome, or an area light in view) shows the effects
best.

1. Open a scene, press **F9**. In the Render window click **Image
   Processing** (or press **I**). Both sections show; every row under
   **Bloom** and **Glare** is greyed until its checkbox is ticked (UIC-117).
   Hovering a greyed row says "Bloom is off" / "Glare is off".
2. Tick **Bloom**. The finished picture glows around its bright parts at
   once, with no new render (the status bar doesn't say "Rendering").
3. Drag **Threshold** down to 0.5 and back with the mini-slider: the glow
   follows the drag; Cmd+Z undoes the whole drag in one step.
4. Change **Intensity**, **Radius** and **Tint** (click the swatch, pick
   a colour): each updates the picture straight away.
5. Tick **Glare**: streaks appear on the brightest points. Try **Streaks**
   2, 4, 6, 8, turn **Angle**, change **Length**.
6. Pick another output in the header's Output menu (e.g. Depth, if the
   scene has one): the picture shows no glow and the tab says Bloom and
   Glare apply to Final Color.
7. Click an older slot in the history strip: it gets the same effects.
   Untick both: every slot shows as rendered.
8. With both on, **Save Image...** as PNG, as OpenEXR and as OpenEXR
   Multilayer, then open each with Load Image...: the PNG has the glow;
   both EXR files are as rendered, with no glow (BG-Q4).
9. Save the scene, close, reopen: the tab shows the same settings.
9b. Select the Final Color output in the Shader Tree: its Properties have
    no Bloom, Bloom Threshold or Bloom Radius rows any more (BG-Q2). A
    scene saved by an older build with the output's Bloom on opens with
    Bloom ticked in this tab.
10. Render again with **F9** while the tab is on: small renders show the
    glow as they fill in; at 4K the picture renders plain and the tab
    says the effects appear when it finishes, then they appear.
11. Type the command once (UIC-164): in the command line or the MCP
    bridge, `render_settings_set` with `{"bloom": true, "bloom_radius": 0.1}`
    ticks Bloom in the tab and changes the picture.
12. With Render Preferences > Save to Pictures on, render: the file in
    Pictures has the glow; the Render window's slot can still be changed.

Speed (release build, 1920 × 1080, one bright pixel): Bloom about 40-80 ms,
Glare with 8 streaks about 110 ms, both about 120 ms
(`cargo test --release --workspace -- post_effects_speed_probe --ignored --nocapture`).
