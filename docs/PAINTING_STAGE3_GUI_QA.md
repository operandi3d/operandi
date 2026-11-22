# 3D painting stage 3: GUI check (more paint tools)

Build: "Operandi PAINTING <sha>.app" in ~/Developer/Operandi-Builds.
Mouse buttons: **left** = primary, **right** = secondary. Alt + drag stays
navigation.

Setup: File > New. Create > Sphere. Texture > Add Color Texture…
(Resolution 1024, Set Color on, black, OK). Texture > Paint Tools > Paint
Tab. With Airbrush (Size 20), paint a few white strokes to have edges.

1. **Smudge.** Click Smudge. **Left-drag** from a white stroke out across
   the black: white is dragged along the stroke. One Cmd+Z removes it.
2. **Blur / Sharp.** Blur: **left-drag** over a stroke edge a few times:
   the edge softens more each time. Sharp: over a softened edge: it gets
   crisper.
3. **Clone.** Click Clone and **left-click** the sphere: the status says
   "Ctrl+click to set the clone source", nothing paints. **Ctrl+left-click**
   a white stroke (the source). **Left-drag** somewhere black: the stroke's
   area copies there; a crosshair shows where it copies from. Release and
   **left-drag** again elsewhere: it continues with the same offset.
4. **Line.** Click Line. **Left-drag** across the sphere: a preview line
   follows; nothing is painted until you release, then a line in the tip's
   size appears and follows the curved surface. Shape: Circle (press is the
   centre), then Rectangle (press and release are corners): outlines only.
5. **Fill.** Paint a grey patch (FG grey). Click Fill, Fill Tolerance 10 %,
   FG red, **left-click** the black: every black pixel turns red, grey and
   white stay; one Cmd+Z undoes it.
6. **Gradients.** Create > Plane, add a Color Texture to it, Linear
   Gradient: **left-drag** across the plane: the whole image ramps FG to BG
   along the drag (preview line while dragging). Radial Gradient: FG at
   the press point fading to BG at the drag's end.
7. **Lasso.** Click Lasso, Style Rectangle, Soft Border 10. **Left-drag** a
   rectangle on the sphere: its outline stays on screen. Click Airbrush,
   tick Use Falloff, paint across the rectangle's edge: paint stops at it,
   fading over 10 pixels outside. Untick Use Falloff: the lasso no longer
   limits. Lasso > Clear Lasso removes it.
