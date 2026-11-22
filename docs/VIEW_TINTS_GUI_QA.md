# Show Weightmaps / Show UV Distortion — GUI checklist

Branch `viewport-weights-distortion` (HT-5, HT-6). For Jason or Codex; one
pass in the QA app.

1. A fresh view's header, after the separator: Weightmaps (small square
   with a soft gradient), UV Distortion (three wavy strokes), Locators,
   Inactive Invisible, Same As Active. The first two are off.
2. Box, then Vertex Map ▸ New Weight Map. Paint the top points to 1 with a
   Vertex Map tool. Click Weightmaps: the top reads red fading to
   grey-green at the bottom, with the light still shading the sides.
   Click again: normal shading. (HT-12)
3. Paint one corner to −1: that corner goes blue. Keep painting: the tint
   follows the brush; Cmd+Z puts it back. (HT-13, HT-15)
4. Select no weight map: the button still toggles, the tooltip adds
   "(no weight map selected)", and nothing is tinted. (HT-14)
5. UV Unwrap the box. Click UV Distortion: neutral. In a UV view scale one
   polygon ×2: it goes blue in 3D, the same colour the UV view shows with
   its own Distortion on; shrink it: red. Polygons with no UVs stay
   untinted. (HT-16, HT-17)
6. Turn both on: where the weight map exists the weight colours show, not
   the distortion. (HT-19)
7. Display ▸ Wireframe: both buttons grey out, with "Not with the
   Wireframe style" in the tooltip. (HT-18)
8. Viewport Properties ▸ Active Meshes: Show Weightmaps and Show UV
   Distortion follow the buttons, and the buttons follow the rows. Quit and
   relaunch: each view kept its settings. (HT-20)
9. A Subdivision (Tab) mesh with weights: tinted per polygon (its average
   weight; provisional, HT-Q4).
