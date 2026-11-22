# 3D painting stage 5: GUI check (inks, Curve Constraint)

Build: "Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds.
Mouse buttons: **left** = primary, **right** = secondary.

Setup: File > New. Create > Sphere. Texture > Add Color Texture…
(Resolution 1024, Set Color on, black). Texture > Paint Tools > Paint Tab,
pick Paintbrush, Size 20. Load a photo as an image (an Image Map's Load
Image…, or File > New Image and paint something on it).

The Paint tab's "Nozzle and Inks" section has four separate ink buttons
(no flyout): Image Ink, Parametric Ink, Random Ink, Projection Ink. Each
**left-click** adds that ink (the button lights, its rows appear at the
bottom of Tool Properties) without changing the tool; clicking it again
takes it off.

1. **Image Ink (acceptance 25).** **Left-click** Image Ink, set its Image
   row to the photo. **Left-click** once in the view: the photo appears over
   the view, see-through, fitted to the view's height, with a circle in the
   centre, a square on the right edge and a circle on the top edge; this
   click paints nothing. **Left-drag** across the sphere: the stroke shows
   the photo's colours under it.
   - **Left-drag** the centre circle: the photo moves; paint again, the
     part of the photo now over the sphere is what's revealed.
   - **Left-drag** the right square out/in: larger/smaller. **Left-drag**
     the top circle left/right: it turns.
   - Keys while it shows: **E** turns it clockwise, **R** back, **D**
     smaller, **F** larger. (These normally run Rotate, Scale, Subdivide and
     Flip; with Image Ink off they do that again.)
   - Make it small (Scale 25 %), untick Repeat, paint past its edge:
     nothing lands outside the picture. Tick Repeat: the picture tiles.
   - Tick Stamp and **left-click** once: the whole picture is laid on the
     visible sphere in one go (one Undo step).
2. **Parametric Ink (acceptance 26).** Take Image Ink off. **Left-click**
   Parametric Ink. Parameter: Slope; untick Use as Mask. FG Color red, BG
   Color blue, Size 300. **Left-drag** over the whole sphere: its top is
   red, its sides blue. Tick Use as Mask, Parameter Incidence Angle: paint
   is full where the sphere faces you and fades to nothing at its outline.
   Displacement and Tangent Pressure are greyed in the Parameter list.
3. **Random Ink (acceptance 27).** **Left-click** Random Ink, Brightness
   30 %, FG grey, Interpolation Step 200 %. **Left-drag**: each dab is a
   slightly different brightness. Hue 50 %: the dabs change colour too.
4. **Projection Ink.** Add a camera. In the camera's properties give it
   a background image. Select the sphere again. **Left-click** Projection
   Ink: the camera's picture shows in the view, 2 m in front of the camera.
   **Left-drag** on the sphere: the picture is painted on as seen from the
   camera. Image Opacity changes how strongly the picture shows; Exposure
   brightens the paint. Tick Stamp, **left-click**: every visible part (or,
   in Polygons mode, the selected polygons) fills in one go. Without a
   camera background image the button is greyed: "Needs a camera
   background image".
5. **Curve Constraint (acceptance 28).** Take the inks off. Make a second
   mesh and draw a curve across the front of the sphere in it (the Curve
   or Pen tool), then select the sphere again so the curve is a background curve.
   Curve Constraint: Background 2D. **Left-drag** wobbling near the curve:
   the stroke follows the curve exactly. Background 3D: the stroke follows
   the curve in space and only paints the sphere where it is within the
   brush of the curve. With no background curve, a note says so and the
   stroke paints freely.
6. **Sculpt tab.** The Inks button in its painting section is greyed:
   "Inks work with the paint tools only".
7. **Undo.** Each stroke, stamp and fill is one Undo step; turning inks on
   and changing their rows is not on the undo list.
