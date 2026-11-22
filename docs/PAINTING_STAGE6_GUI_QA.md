# 3D painting stage 6: GUI check (layout, set, menus, preferences, UV painting, polish)

Build: "Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds.
Mouse buttons: **left** = primary, **right** = secondary.

Setup: File > New. Create > Sphere. Texture > Add Color Texture…
(Set Color on, black).

1. **Paint layout (acceptance 29).** Open the layout switcher. "Paint" is
   listed after the shipped layouts and isn't starred. Pick it. The
   toolbox's tabs become Sculpt Tools, Paint Tools, Vertex Map Tools and
   Utilities, with Paint Tools open. The 3D view is in Texture shading, so
   the black texture shows. A dark strip runs along the bottom of the 3D
   view. With no presets it says how to make one.
   - On Paint Tools, pick a Hard tip and click Save Brush Preset. A
     thumbnail appears in the strip within two seconds. Pick the Smooth
     tip, then **left-click** the thumbnail: the tip goes back to Hard and
     nothing is painted.
   - Switch to Model: the usual tabs come back and the strip goes.
2. **Utilities tab.** **Left-click** Utilities. There are three sections:
   - Paint Commands: Add Fur Texture is greyed.
   - Sculpt Commands: only Relax Mesh works; the others are greyed "Not
     in Operandi's sculpting yet".
   - Image Processing: Convert to Vector Displacement is greyed.
   Vertex Map Tools holds the vertex map brushes.
3. **Menus.** File has New Image…, New Image Sequence… (greyed), Save
   Image, Save Image As… and Save All Images. Texture has Paint Tools and
   an Image Processing submenu (Invert, Clear, Expand UV Seams, Convert to
   Vector Displacement, Set Resolution…, Flatten Layers).
4. **Preferences.** Open the Images & Painting page and change these:
   - Default New Image Resolution 512: Add Color Texture… now offers 512.
   - Default Foreground Color Black: every paint tool's FG turns black.
   - Maximum Brush Size 40: a **right-drag** sizing the tip stops at 40.
   - Always Display Images as Icons: the Images window shows thumbnails.
   Under Defaults, open Default Tools and set Default Brush to Sphere. Turn
   the paint tools off, then pick Paintbrush: its tip is Sphere.
5. **UV view painting (acceptance 30).** Switch to the UV layout and pick
   Paintbrush (Texture > Paint Tools). The black image shows behind the UV
   map. **Left-drag** across the UV tile: white paint appears in the UV
   view and on the sphere in the 3D view while you drag. Release: Edit >
   Undo removes the whole stroke. Fill: one **left-click** fills the image.
   Linear Gradient: **left-drag** across the tile. **Alt+left-drag** still
   pans. With no paint tool on, the UV view selects as before.
6. **Images list and Image Viewer.** Texture > Images: each row shows the
   name, a * when unsaved, its size and "RGBA 8-bit". **Double-click** a
   row: the Image Viewer opens. Paint in the 3D view and the viewer updates
   as you drag. A Photoshop image has a PSD Layer picker in the viewer.
7. **Flyouts.** On the Paint tab, Fill, Blur, Procedural and Inks are
   flyouts (a small corner triangle). **Hold left** or **right-click** one
   to open its list, then pick Radial Gradient. The face now shows Radial
   and keeps it. Pick Linear Gradient, turn it off, then
   **Alt+left-click** the face: Radial Gradient turns on.
8. **Colour pick.** With Paintbrush on, paint some red. Then
   **Ctrl+Shift+left-click** the red: FG Color becomes red. Note: Ctrl,
   not Cmd, on a Mac.
9. **Lasso handles.** Click Lasso, **left-drag** a rectangle on the
   sphere, click Lasso again and pick Paintbrush. A small square sits at
   the lasso's centre and a circle at its lower-right corner.
   **Left-drag** the square to move the lasso, or the circle to size it.
   Then orbit the view (**Alt+left-drag**): the outline goes and a note
   says "Lasso: fixed to the model". With Use Falloff on, painting is
   still limited to the part of the sphere the lasso covered.
10. **PSD layer in the view.** Load a .psd as an Image Map and set its PSD
    Layer to one layer: the 3D view shows that layer, not the composite.
