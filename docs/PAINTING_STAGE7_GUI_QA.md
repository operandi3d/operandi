# 3D painting stage 7: GUI check (sculpt extras)

Build: "Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds.
Mouse buttons: **left** = primary, **right** = secondary.

Setup: File > New. Create > Sphere. Open the Sculpt tab. Its painting
section (below the brushes) has the tips, Nozzle, Inks and the Mask
buttons.

1. **Texture tip (A1).** Load a greyscale picture with a mid-grey border
   (for example from Utilities > Geometry to Brush, step 6). Pick Push.
   **Left-click** Texture, then in Tool Properties set Image to the
   picture, tick Use as Mask, and set Offset to 50 %. **Left-click** once
   on the sphere: the picture's pattern is pushed in, and the grey border
   leaves no step. Set Offset to 0 % and click again: the grey pushes too.
2. **Procedural / Sphere.** **Hold left** on Procedural (or
   **right-click** it) and pick Sphere. **Left-drag**: rounded ridges.
   Procedural gives rough, broken strokes. Click Smooth tip on the brush
   row and the extra tip goes away.
3. **Parametric Ink (A2).** Pick Clay. In the Inks flyout pick Parametric
   Ink. Set Parameter to Slope and tick Use as Mask. **Left-drag** over
   the top and down the side: only the up-facing areas change. Projection
   Ink is greyed here ("paint tools only").
4. **Mask (A3).** **Left-click** Mask Paint and **left-drag** over half the
   sphere: it turns red. Pick Push and **left-drag** across: the red half
   doesn't move, and pale (partly masked) areas move partly. Tick Invert
   Mask (Tool Properties): now only the red part moves. **Ctrl+left-drag**
   with Mask Paint erases. Mask Smooth softens the edge. Clear Mask removes
   it (Edit > Undo brings it back). Save, close and reopen the scene: no
   mask. Bake Falloff turns an active falloff into the mask.
5. **Nozzle.** **Left-click** Nozzle and turn on jitter in its rows: sculpt
   dabs scatter. With a pen, pressure changes strength and size.
6. **Geometry to Brush (A5).** Make a plane, Subdivide it a few times and
   pull a bump up in the middle with Push. In Polygon mode select the
   bump's polygons (an open patch). Paint layout > Utilities > Geometry to
   Brush: pick Height Field and OK. The sculpt tip is now Texture with the
   new "Brush from Geometry" picture. Click once on another plane with
   Push: the same bump appears. On a closed cube it says it needs an open
   patch.
7. **Starter brushes (A4).** Paint layout: the strip starts with seven
   brushes (Dynamic Smooth, Jitter, Scattered Dots, Crumb Coarse, Crumb
   Fine, Cellular, Dots). With Paintbrush on, **left-click** each one and
   paint: the look changes and the tool stays. Pick Clay and do the same.
   The crumb brushes add their picture to the Images list.
8. **Constrain to Background.** Put a second mesh above the sphere, turn
   on Constrain to Background, and Push the sphere up into it: the pushed
   points stop on the other mesh.
9. **Default build (A6).** In a normal (non-painting) build the Sculpt tab
   and sculpting are exactly as before.
10. **Paint on a small Sub-D mesh.** Add a sphere, press Tab, add a
    texture and paint with Paintbrush: the paint shows under the brush
    while you drag, not only after Sub-D is turned off.
11. **No highlighting while painting.** Select some polygons, turn on
    Paintbrush (or a sculpt or mask brush): the selected tint and the
    hover highlight go away; the wire and brush ring stay. Turn on any
    other tool (e.g. Shear): the selection shows again.
12. **Stroke start.** On a Sub-D sphere with a 2048 texture, make several
    short strokes in a row: each starts the moment you press, no hitch.
13. **Texture Locator while painting.** With the texture's layer selected
    (its yellow locator shows), turn on Paintbrush: the locator goes. Texture
    > Paint Tools > Show Texture Locators while Painting brings it back.
