<!-- help-ids: Image Viewer: Image Processing Tab ; template: panel ; since: 0.1.0 -->
# Image Processing: Bloom and Glare

The **Image Processing** tab of the Render window adds a glow (**Bloom**) and star streaks (**Glare**) around the brightest parts of a finished render.

Both work on the picture after it is rendered. They don't change the render itself, so you can try settings on a finished picture and see the result straight away, without rendering again. They are off in every new scene.

## Parts of the tab

Open the Render window (**Render > Open Render Window**), then click **Image Processing** above its side panel (or press **I** while the window has the focus).

### Bloom

A soft glow that spreads out from bright areas, like light blooming in a camera lens.

| Name | What it does | Default |
|---|---|---|
| **Bloom** | Turns the glow on. The rows below are greyed while it is off. | Off |
| **Threshold** | How bright a pixel must be before it glows. 1 is white. Lights, the sun and bright highlights are often far above 1, so they glow while a white wall doesn't. | 1 |
| **Intensity** | How much of the light over the threshold spreads into the glow. At 100 % the glow carries as much light again as was over the threshold. | 50 % |
| **Radius** | How far the glow reaches, as a share of the picture's width. Because it is a share, a glow looks the same at any render size. | 5 % |
| **Tint** | The glow's colour. White keeps each light's own colour. | White |

### Glare

Straight streaks out of bright points, like a star filter on a camera.

| Name | What it does | Default |
|---|---|---|
| **Glare** | Turns the streaks on. The rows below are greyed while it is off. | Off |
| **Threshold** | How bright a point must be before it streaks. | 2 |
| **Intensity** | How much of the light over the threshold goes into the streaks. | 30 % |
| **Streaks** | Rays around each bright point: **2** is one line through it, **4** a cross, **6** and **8** stars. | 4 |
| **Angle** | Turns the streaks. At 0 ° the first ray points to the right. | 45 ° |
| **Length** | How far the streaks reach, as a share of the picture's width. They fade out along it. | 20 % |
| **Tint** | The streaks' colour. | White |

## Common tasks

### Add a glow to a finished render

1. Render with **F9**.
2. In the Render window, click the **Image Processing** tab.
3. Tick **Bloom**.
4. Lower **Threshold** until the parts you want glowing glow, then set **Intensity** and **Radius** to taste.

The picture updates as you change each value. Each change is one undo step, and one drag of a value is one step for the whole drag.

### Add star streaks to highlights

1. Tick **Glare**.
2. Pick the number of **Streaks**, then turn them with **Angle**.
3. Raise **Threshold** if too many small highlights streak.

## Good to know

- **Only Final Color.** Bloom and Glare change the Final Color picture. Depth, Normal and the other outputs keep their true numbers, so they stay usable for compositing.
- **Saved with the scene.** The settings are part of the scene's render settings and are saved with it.
- **The one place for Bloom.** The Final Color render output no longer has Bloom rows of its own. A scene saved with them on opens with **Bloom** ticked here instead, at the same **Threshold**, full **Intensity** and about the same reach.
- **In saved pictures.** **Save Image...** in a picture format (PNG, JPEG, TIFF, Targa, BMP) writes the picture as the window shows it, Bloom and Glare included, and so do renders saved to your Pictures folder or to a path a script gives. Float files (OpenEXR, OpenEXR Multilayer and Radiance HDR) keep the numbers as rendered, without Bloom and Glare, so they stay clean for compositing. The Render window's history keeps every render as it was rendered, so you can change the settings on an older render later.
- **While it renders.** Small pictures show the effects as they render. On big pictures, where working them out takes longer, the render shows without them and they appear when it finishes.
- **Every slot.** The tab's settings belong to the open scene and apply to whichever picture the window shows.
- **From a script.** `render_settings_set` takes `bloom`, `bloom_threshold`, `bloom_intensity`, `bloom_radius`, `bloom_tint`, `glare`, `glare_threshold`, `glare_intensity`, `glare_streaks`, `glare_angle`, `glare_length` and `glare_tint`, and `render_settings_get` reports them. Intensities, Radius and Length are fractions there (0.5 is 50 %), the angle is in radians, and tints are linear colours.

## Related

- [Render menu](../menus/render.md)
- [Scripting and the test bridge](../scripting/README.md)
